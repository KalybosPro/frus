# Milestone 352 — The first pixel

> *"L'app ne se démarre pas correctement comme une app Flutter pouvait le faire."*

It does not, and the reason is a file the reference ships and this framework did not.

Android paints the window from the moment it opens it, which is well before the first
frame exists. What it paints then comes from the activity's theme, and the theme was
`@android:style/Theme.DeviceDefault.NoActionBar` — the platform's, whose background on a
device in light mode is **white**. So the demo, which is dark, opened with a full-screen
white flash and then snapped to dark.

Measured rather than guessed: `am start -W` reports 275 ms to the first frame, which is
fast, and a screencap taken in that window is a solid white page. The application was not
slow to start. It was showing the wrong thing while it started.

## What the reference does

`flutter create` puts a `LaunchTheme` in `android/app/src/main/res/values/styles.xml`
whose `windowBackground` is a drawable you are meant to edit, and points the activity at
it. That is the whole mechanism: no code, no splash-screen API, one resource.

frus ships the same thing now — in the demo, in the three example applications, and in
the `cargo generate` template, so a new application has it from the first build:

```xml
<resources>
    <color name="launch_background">#121418</color>

    <style name="LaunchTheme" parent="@android:style/Theme.DeviceDefault.NoActionBar">
        <item name="android:windowBackground">@color/launch_background</item>
    </style>
</resources>
```

`#121418` is `Theme::dark()`'s background, which is what a frus application uses unless
it says otherwise. The file carries a comment saying to change it, and `getting-started`
says so too, including the `values-night/` copy an application that follows the system
wants.

`cargo-apk` already had `resources`, so the manifest side is two lines.

## Two things that were tried and are not there

The window opens with a white strip along the navigation bar, which `windowBackground`
does not reach. The obvious fix is the theme's own `navigationBarColor`, and:

- **`statusBarColor` and `navigationBarColor` do nothing on their own.** The platform
  only honours them once the window is told to draw the system bars' backgrounds.
- **`windowDrawsSystemBarBackgrounds`, which tells it to, brings the white flash back.**
  Measured on the device: with that flag the launch background stops being used at all
  and the full white page returns.

So the strip stays, and both findings are written into the resource file rather than
rediscovered. A strip of navigation bar for the length of an opening animation is the
smaller of the two by a wide margin.

## Verified on the device

`XMJNW19B23011768`, release build, cold start after `force-stop`:

- **before** — a full-screen white page, then the dark application;
- **after** — the window opens dark and stays dark; the only white left is the navigation
  bar strip during the opening animation.

And the running application is unchanged: the app bar sits under the status bar, the
bottom bar above the navigation bar, so the safe area is exactly where it was. That was
the thing to check — milestone 288 was a theme changing the content rect out from under
the shell.

## Left

- **A launch background is a colour here, not a drawable.** The reference lets you put
  your icon on it, which is the difference between "the app is starting" and "*this* app
  is starting". A `layer-list` would do it and needs the icon as a resource.
- **Nothing enforces that the colour matches.** An application that changes its theme and
  forgets this file gets its old flash back, quietly. The framework knows both numbers at
  build time and could say so.
- **Desktop and web have the same question** and have not been asked it: a winit window
  is painted by the platform before the first frame too.
