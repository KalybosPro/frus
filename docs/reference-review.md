# Review against the reference

Every frus widget that has a counterpart in the reference is reviewed for **what it does**:
its behaviour, appearance, defaults, states and animations, against the reference's source,
with file and line cited. Names of properties are not the point. Each review that finds
differences fixes them in a milestone of its own; one that finds none says so here.

Widgets without a counterpart (charts, `Kanban`, `Rating`, `Skeleton`, `Timeline`…) are
frus's own and are not part of this review.

Status: **done** (milestone), **to review**.

## 1. Buttons

| frus | reference | source | status |
|---|---|---|---|
| `Button` | `ElevatedButton, FilledButton, FilledButton.tonal, OutlinedButton, TextButton` | `material/elevated_button.dart …` | **done** (J632) |
| `IconButton` | `IconButton` | `material/icon_button.dart` | to review |
| `FloatingActionButton` | `FloatingActionButton` | `material/floating_action_button.dart` | to review |
| `SegmentedButton` | `SegmentedButton` | `material/segmented_button.dart` | to review |
| `Segment` | `ButtonSegment` | `material/segmented_button.dart` | to review |
| `ToggleButtons` | `ToggleButtons` | `material/toggle_buttons.dart` | to review |
| `BackButton` | `BackButton` | `material/action_buttons.dart` | to review |
| `CloseButton` | `CloseButton` | `material/action_buttons.dart` | to review |
| `DrawerButton` | `DrawerButton` | `material/action_buttons.dart` | to review |
| `EndDrawerButton` | `EndDrawerButton` | `material/action_buttons.dart` | to review |
| `InkWell` | `InkWell` | `material/ink_well.dart` | to review |

## 2. Container

| frus | reference | source | status |
|---|---|---|---|
| `Container` | `Container` | `widgets/container.dart` | **done** (J633) |

## 3. Form controls

| frus | reference | source | status |
|---|---|---|---|
| `Checkbox` | `Checkbox` | `material/checkbox.dart` | **done** (J634) |
| `Radio` | `Radio` | `material/radio.dart` | **done** (J635) |
| `RadioGroup` | `RadioGroup` | `widgets/radio_group.dart` | to review |
| `Switch` | `Switch` | `material/switch.dart` | to review |
| `Slider` | `Slider` | `material/slider.dart` | to review |
| `RangeSlider` | `RangeSlider` | `material/range_slider.dart` | to review |
| `TextField` | `TextField` | `material/text_field.dart` | to review |
| `DropdownButton` | `DropdownButton` | `material/dropdown.dart` | to review |
| `DropdownMenu` | `DropdownMenu` | `material/dropdown_menu.dart` | to review |
| `Autocomplete` | `Autocomplete` | `material/autocomplete.dart` | to review |
| `CheckboxListTile` | `CheckboxListTile` | `material/checkbox_list_tile.dart` | to review |
| `RadioListTile` | `RadioListTile` | `material/radio_list_tile.dart` | to review |
| `SwitchListTile` | `SwitchListTile` | `material/switch_list_tile.dart` | to review |

## 4. Progress and feedback

| frus | reference | source | status |
|---|---|---|---|
| `LinearProgressIndicator` | `LinearProgressIndicator` | `material/progress_indicator.dart` | **done** (J629) |
| `CircularProgressIndicator` | `CircularProgressIndicator` | `material/progress_indicator.dart` | **done** (J630) |
| `RefreshIndicator` | `RefreshIndicator` | `material/refresh_indicator.dart` | to review |
| `SnackBar` | `SnackBar` | `material/snack_bar.dart` | to review |
| `ScaffoldMessenger` | `ScaffoldMessenger` | `material/scaffold.dart` | to review |
| `BannerSurface` | `MaterialBanner` | `material/banner.dart` | to review |
| `Tooltip` | `Tooltip` | `material/tooltip.dart` | to review |
| `Badge` | `Badge` | `material/badge.dart` | to review |

## 5. Surfaces and structure

| frus | reference | source | status |
|---|---|---|---|
| `AppBar` | `AppBar` | `material/app_bar.dart` | to review |
| `Scaffold` | `Scaffold` | `material/scaffold.dart` | to review |
| `BottomAppBar` | `BottomAppBar` | `material/bottom_app_bar.dart` | to review |
| `Card` | `Card` | `material/card.dart` | to review |
| `Divider` | `Divider` | `material/divider.dart` | to review |
| `VerticalDivider` | `VerticalDivider` | `material/divider.dart` | to review |
| `ListTile` | `ListTile` | `material/list_tile.dart` | to review |
| `Chip` | `Chip` | `material/chip.dart` | to review |
| `CircleAvatar` | `CircleAvatar` | `material/circle_avatar.dart` | to review |
| `Dialog` | `Dialog` | `material/dialog.dart` | to review |
| `Alert` | `AlertDialog` | `material/dialog.dart` | to review |
| `BottomSheet` | `BottomSheet` | `material/bottom_sheet.dart` | to review |
| `DraggableScrollableSheet` | `DraggableScrollableSheet` | `widgets/draggable_scrollable_sheet.dart` | to review |
| `Drawer` | `Drawer` | `material/drawer.dart` | to review |
| `DrawerHeader` | `DrawerHeader` | `material/drawer_header.dart` | to review |
| `UserAccountsDrawerHeader` | `UserAccountsDrawerHeader` | `material/user_accounts_drawer_header.dart` | to review |
| `ExpansionTile` | `ExpansionTile` | `material/expansion_tile.dart` | to review |
| `ExpansionPanelList` | `ExpansionPanelList` | `material/expansion_panel.dart` | to review |
| `GridTile` | `GridTile` | `material/grid_tile.dart` | to review |
| `GridTileBar` | `GridTileBar` | `material/grid_tile_bar.dart` | to review |
| `Stepper` | `Stepper` | `material/stepper.dart` | to review |
| `DataTable` | `DataTable` | `material/data_table.dart` | to review |
| `CarouselView` | `CarouselView` | `material/carousel.dart` | to review |
| `Icon` | `Icon` | `widgets/icon.dart` | to review |
| `Image` | `Image` | `widgets/image.dart` | to review |
| `ImageIcon` | `ImageIcon` | `widgets/image_icon.dart` | to review |
| `Text` | `Text` | `widgets/text.dart` | to review |
| `RichText` | `RichText` | `widgets/basic.dart` | to review |
| `SelectionToolbar` | `AdaptiveTextSelectionToolbar` | `material/adaptive_text_selection_toolbar.dart` | to review |

## 6. Navigation and menus

| frus | reference | source | status |
|---|---|---|---|
| `NavigationBar` | `NavigationBar` | `material/navigation_bar.dart` | to review |
| `NavigationRail` | `NavigationRail` | `material/navigation_rail.dart` | to review |
| `NavigationDrawer` | `NavigationDrawer` | `material/navigation_drawer.dart` | to review |
| `TabBar` | `TabBar` | `material/tabs.dart` | to review |
| `Tab` | `Tab` | `material/tabs.dart` | to review |
| `TabPageSelector` | `TabPageSelector` | `material/tabs.dart` | to review |
| `MenuAnchor` | `MenuAnchor` | `material/menu_anchor.dart` | to review |
| `MenuBar` | `MenuBar` | `material/menu_anchor.dart` | **done** (J636) |
| `PopupMenuButton` | `PopupMenuButton` | `material/popup_menu.dart` | to review |
| `SearchBar` | `SearchBar` | `material/search_anchor.dart` | to review |
| `SearchAnchor` | `SearchAnchor` | `material/search_anchor.dart` | to review |
| `DatePicker` | `CalendarDatePicker` | `material/calendar_date_picker.dart` | to review |
| `TimePicker` | `TimePickerDialog` | `material/time_picker.dart` | to review |
| `Navigator` | `Navigator` | `widgets/navigator.dart` | to review |
| `Hero` | `Hero` | `widgets/heroes.dart` | to review |

## 7. Scrolling and lists

| frus | reference | source | status |
|---|---|---|---|
| `ListView` | `ListView` | `widgets/scroll_view.dart` | to review |
| `GridView` | `GridView` | `widgets/scroll_view.dart` | to review |
| `PageView` | `PageView` | `widgets/page_view.dart` | to review |
| `SingleChildScrollView` | `SingleChildScrollView` | `widgets/single_child_scroll_view.dart` | to review |
| `ListWheel` | `ListWheelScrollView` | `widgets/list_wheel_scroll_view.dart` | to review |
| `ReorderableList` | `ReorderableList` | `widgets/reorderable_list.dart` | to review |
| `Dismissible` | `Dismissible` | `widgets/dismissible.dart` | to review |
| `InteractiveViewer` | `InteractiveViewer` | `widgets/interactive_viewer.dart` | to review |
| `StickyHeader` | `— (sliver pinned header)` | `widgets/sliver_persistent_header.dart` | to review |
| `Tree` | `TreeSliver` | `widgets/sliver_tree.dart` | to review |
| `Draggable` | `Draggable` | `widgets/drag_target.dart` | to review |
| `DragTarget` | `DragTarget` | `widgets/drag_target.dart` | to review |

## 8. Layout

| frus | reference | source | status |
|---|---|---|---|
| `Padding` | `Padding` | `widgets/basic.dart` | to review |
| `Aligned` | `Align` | `widgets/basic.dart` | to review |
| `Center` | `Center` | `widgets/basic.dart` | to review |
| `SizedBox` | `SizedBox` | `widgets/basic.dart` | to review |
| `ConstrainedBox` | `ConstrainedBox` | `widgets/basic.dart` | to review |
| `LimitedBox` | `LimitedBox` | `widgets/basic.dart` | to review |
| `OverflowBox` | `OverflowBox` | `widgets/basic.dart` | to review |
| `SizedOverflowBox` | `SizedOverflowBox` | `widgets/basic.dart` | to review |
| `ConstraintsTransformBox` | `ConstraintsTransformBox` | `widgets/basic.dart` | to review |
| `FractionallySizedBox` | `FractionallySizedBox` | `widgets/basic.dart` | to review |
| `AspectRatio` | `AspectRatio` | `widgets/basic.dart` | to review |
| `Baseline` | `Baseline` | `widgets/basic.dart` | to review |
| `FittedBox` | `FittedBox` | `widgets/basic.dart` | to review |
| `Row` | `Row` | `widgets/basic.dart` | to review |
| `Column` | `Column` | `widgets/basic.dart` | to review |
| `Flex` | `Flex` | `widgets/basic.dart` | to review |
| `Stack` | `Stack` | `widgets/basic.dart` | to review |
| `IndexedStack` | `IndexedStack` | `widgets/indexed_stack.dart` | to review |
| `Spacer` | `Spacer` | `widgets/spacer.dart` | to review |
| `OverflowBar` | `OverflowBar` | `widgets/overflow_bar.dart` | to review |
| `Table` | `Table` | `widgets/table.dart` | to review |
| `CustomMultiChildLayout` | `CustomMultiChildLayout` | `widgets/basic.dart` | to review |
| `LayoutBuilder` | `LayoutBuilder` | `widgets/layout_builder.dart` | to review |
| `ThemeBuilder` | `Builder + Theme.of` | `widgets/basic.dart` | to review |
| `SafeArea` | `SafeArea` | `widgets/safe_area.dart` | to review |
| `RotatedBox` | `RotatedBox` | `widgets/basic.dart` | to review |
| `Transform` | `Transform` | `widgets/basic.dart` | to review |
| `FractionalTranslation` | `FractionalTranslation` | `widgets/basic.dart` | to review |
| `Placeholder` | `Placeholder` | `widgets/placeholder.dart` | to review |
| `OverlayPortal` | `OverlayPortal` | `widgets/overlay.dart` | to review |

## 9. Animation

| frus | reference | source | status |
|---|---|---|---|
| `AnimatedAlign` | `AnimatedAlign` | `widgets/implicit_animations.dart` | to review |
| `AnimatedCrossFade` | `AnimatedCrossFade` | `widgets/animated_cross_fade.dart` | to review |
| `AnimatedDefaultTextStyle` | `AnimatedDefaultTextStyle` | `widgets/implicit_animations.dart` | to review |
| `AnimatedFractionallySizedBox` | `AnimatedFractionallySizedBox` | `widgets/implicit_animations.dart` | to review |
| `AnimatedPositioned` | `AnimatedPositioned` | `widgets/implicit_animations.dart` | to review |
| `AnimatedSize` | `AnimatedSize` | `widgets/animated_size.dart` | to review |
| `AnimatedSwitcher` | `AnimatedSwitcher` | `widgets/animated_switcher.dart` | to review |
| `DecoratedBoxTransition` | `DecoratedBoxTransition` | `widgets/transitions.dart` | to review |
| `SizeTransition` | `SizeTransition` | `widgets/transitions.dart` | to review |

## 10. Painting, effects, input, semantics

| frus | reference | source | status |
|---|---|---|---|
| `CustomPaint` | `CustomPaint` | `widgets/basic.dart` | to review |
| `ClipRect` | `ClipRect` | `widgets/basic.dart` | to review |
| `ClipRRect` | `ClipRRect` | `widgets/basic.dart` | to review |
| `ClipOval` | `ClipOval` | `widgets/basic.dart` | to review |
| `ClipPath` | `ClipPath` | `widgets/basic.dart` | to review |
| `BackdropFilter` | `BackdropFilter` | `widgets/basic.dart` | to review |
| `BackdropGroup` | `BackdropGroup` | `widgets/basic.dart` | to review |
| `ColorFiltered` | `ColorFiltered` | `widgets/color_filter.dart` | to review |
| `ImageFiltered` | `ImageFiltered` | `widgets/image_filter.dart` | to review |
| `ShaderMask` | `ShaderMask` | `widgets/basic.dart` | to review |
| `AbsorbPointer` | `AbsorbPointer` | `widgets/basic.dart` | to review |
| `IgnorePointer` | `IgnorePointer` | `widgets/basic.dart` | to review |
| `Offstage` | `Offstage` | `widgets/basic.dart` | to review |
| `Visibility` | `Visibility` | `widgets/indexed_stack.dart` | to review |
| `ExcludeSemantics` | `ExcludeSemantics` | `widgets/basic.dart` | to review |
| `Semantics` | `Semantics` | `widgets/basic.dart` | to review |
| `AnnotatedRegion` | `AnnotatedRegion` | `widgets/annotated_region.dart` | to review |
| `KeyboardListener` | `KeyboardListener` | `widgets/keyboard_listener.dart` | to review |
