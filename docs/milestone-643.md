# Milestone 643 — The system paints the title bar's line

## Objective

Under J640 and J642, frus painted the whole title bar's line on Windows: its surface in the
caption's colours, and the window's three buttons where the system has them. The maintainer
asked for the system's own line instead: "que ça soit un truc natif". Asked which native
they meant, they chose the system painting the line. Windows draws its caption backdrop,
including Windows 11's wallpaper-tinted Mica, and its own three buttons with their own hover
and press. frus draws only what is the application's on the line: the window's icon and the
menu bar.

## Why frus had painted it

Windows keeps its caption buttons when an application extends the frame into its window
("Custom Window Frame Using DWM"), but draws them **under** the window's content. J640's
prototype found them hidden by frus's GPU-rendered content, which is opaque: a swap chain
made from the window's handle has no transparency. So J640 painted look-alikes where the
system had them.

A GPU frame can be composed with what is behind the window only through
**DirectComposition**. wgpu 30 offers it on Direct3D 12
(`Dx12SwapchainKind::DxgiFromVisual`), with premultiplied alpha. The reference leaves the
title bar to the system and draws nothing on it: its Windows runner only sets the caption
light or dark.

## What was done

- **A see-through renderer** (`Renderer::see_through`, `frus-gpu`): Direct3D 12, presented
  through a DirectComposition visual made from the window, with premultiplied composition.
  Its frames start transparent. frus's blending writes premultiplied colour over a
  transparent start (`ALPHA_BLENDING` composes alpha as *over*), and its layers already
  composite premultiplied. Where it cannot be had (no Direct3D 12 adapter, no premultiplied
  composition, not Windows), it fails, and the shell makes the ordinary renderer.
- **The window's background** (`backdrop()`, the old clear colour as a scene colour) is
  painted by the shell under every see-through frame, everywhere but the line the system
  paints.
- **`TitleBar::system_paints`.** Where it is set, the row paints no surface (unless told
  one) and no buttons; it keeps their room clear. The menu bar's surface is transparent, and
  its highlight is a wash of its words, at the opacity that reads as the opaque highlight
  would over the system's caption. frus blends a wash in linear light, where 12 % of white
  over a dark caption reads as 40 %: measured, the open word's box was `#7C7E7D` on
  `#1E2120`, and now it is the 12 % it was meant to be.
- **The system's backdrop on the extended frame**: without one, the frame extended into the
  window is black. The shell asks for the main window's backdrop
  (`DWMWA_SYSTEMBACKDROP_TYPE`, or the first Windows 11's own attribute), dark or light as
  the window is (`DWMWA_USE_IMMERSIVE_DARK_MODE`, what the reference's runner sets). Where
  the system does not take it (Windows 10), the line stays frus-painted, as in J642.
- **The window's own drawing surface is black** (`WM_ERASEBKGND`). That surface, kept for
  drawing with the system's older calls, lies under the composition visual. Where the line is
  transparent it showed through: white where the window had been, black where it had grown.
  Over the extended frame, black is what lets the frame show.

## Three bugs found on the machine

- **Maximized, the content started off the screen** (since J640). The window procedure
  moved the client down by the frame when maximized, measuring the frame against the
  window's rectangle. While that message is handled, the rectangle is still the restored
  one, so the offset came out negative, was clamped to zero, and the top of the line was
  off the screen. Frus-painted buttons were cut the same way, so it went unseen. The
  system's own buttons were not, and stood lower than the words. The offset is now measured
  against the proposed rectangle.
- **Maximized, the system's buttons sat lower than the words.** The frame was extended by
  the caption *and* the resizing border, which is off the screen when maximized, so the
  system centred its buttons in a line taller than the one on the screen. Now the frame is
  extended by the caption alone when maximized, again on every resize, and the content's
  line ends where the system's buttons do. Measured: the words' centre and the glyphs' are
  within a pixel, as on a standard window's caption.
- **An inactive window's words stayed bright**, because nothing asked for a frame when the
  window lost the focus. Now the words take the inactive caption's grey, `#797979`, the
  system's own.

## Checked on Windows 11

On the maintainer's machine (Intel Iris Xe, Direct3D 12), driven by real clicks:

- the line is the system's: Mica (`#1D2120` to `#1F2022`, tinted by the wallpaper) and its
  buttons; the pointer over "minimize" gives the system's own hover (`#2C2D2E`);
- File opens its menu, with a quiet highlight;
- maximizing by the system's button, by a double click on the line and by the system's
  call; restoring;
- inactive: the system's solid `#202020` and dimmed buttons, and the words `#797979`.

## Tests

- `where_the_system_paints_the_line_the_application_leaves_it_clear`: nothing opaque of the
  application's on the line, the system's ink on the words, no painted buttons, the bar where
  it was.
- `the_highlight_on_the_system_s_line_reads_as_on_its_caption`: blended in linear light over
  a dark and a light caption, the wash gives the caption moved 12 % toward the words.
- `the_background_leaves_the_system_s_line_clear` (shell): the window's background under
  everything but the line, opaque, and the scene over it unchanged.
- The DirectComposition path itself runs only on Windows with Direct3D 12, and CI is
  Linux. It was exercised on the maintainer's machine as above, and linted there with
  clippy.
- `a_see_through_bar_s_highlight_is_a_wash_of_its_words` (`menubar.rs`): a bar with a
  transparent surface lights its open word with a wash of its words.

## Mutation testing

Seven mutants, seven killed: the buttons' room left clear, the system's caption kept off a
line the system paints, the transparent surface, the highlight set to the wash, the wash's
opacity, the menu bar's wash over a transparent surface, and the background leaving the
line clear. The first full run also caught a wrong expectation in a test of its own: black
over a light caption needs *more* than 12 % to read as 12 %, not less.
