# lm-modal TODO

## Floating Window (Hyprland)
- [ ] Add window rule for floating by default
- [ ] Consider size/position hints
- [ ] Update nixos-config with Hyprland window rule

## Keyboard Shortcuts
- [x] Ctrl+Enter → Send message
- [x] Enter → Insert newline (multiline default)
- [x] Shift+C → Copy response to clipboard
- [x] Esc → Clear session
- [x] Tab → Toggle multiturn mode

## Stylix Theming
- [ ] Add Stylix NixOS module integration
- [ ] Export theme colors via config file
- [ ] Read Stylix-generated colors in app
- [ ] Apply to egui UI (background, text, accent)

## History Tree View
- [x] Design: Tree structure similar to pi-coding-agent
  - Each session stored as JSONL
  - Branches for alternative responses
  - Root = first prompt, children = follow-ups
- [ ] Add history button to UI
- [ ] Create tree viewer screen/modal
- [ ] Allow selecting/continuing from history nodes
- [ ] Show timestamps and truncated previews

## Multiturn Toggle (Tab)
- [x] State: single-turn (default) vs multi-turn
- [x] Single-turn: Each request is independent
- [x] Multi-turn: Maintain conversation context
- [x] Visual indicator of current mode
- [x] Tab key to toggle

## History Management
- [x] Default: Temporary session (no persistence)
- [x] Auto-stash to backup on certain actions
  - [x] Rotation: Keep last 10 backup sessions
  - [x] Location: `~/.local/share/lm-modal/backups/`
- [x] Backup triggers:
  - Window close with content
  - Before clearing
- [x] Format: JSON per session
- [x] Backup rotation: Delete oldest when >10

---

## Implementation Order

1. ~~Keyboard shortcuts~~ ✓ (foundation for other features)
2. ~~Multiturn toggle~~ ✓ (needs mode state)
3. ~~History management~~ ✓ (backup/rotation)
4. History tree UI (visual)
5. Stylix theming (visual polish)
6. Floating window (nixos-config)