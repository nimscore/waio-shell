# simple-popup

Small example that demonstrates:

- Showing a popup with `shell.popups().builder(...)`
- Content-based sizing (manual) using a Slint `Timer` to call `resize_popup(width, height)`
- Multiple simultaneous popups (one at cursor, one centered)

This example intentionally keeps the popup “resize callback” pattern, since the popup window size cannot be tracked automatically.

## Run

From the layer-shika workspace root:

```bash
cargo run -p simple-popup
```

## Notes

- Popups are transient windows intended for menus/tooltips/dialogs.
- For `PopupSize::Content`, the popup starts small and then requests a resize once layout stabilizes.
- If you don’t see the popup, make sure your compositor supports `xdg-shell` popups and that the surface is focused.
