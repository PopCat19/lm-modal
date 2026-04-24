# lm-modal TODO

## Floating Window (Hyprland)
- [ ] Add window rule for floating by default
- [ ] Consider size/position hints
- [ ] Update nixos-config with Hyprland window rule

## Keyboard Shortcuts
- [ ] Enter → Send message (commit)
- [ ] Shift+Enter → Insert newline in input
- [ ] Shift+C → Copy response to clipboard
- [ ] Esc → Close window / Clear
- [ ] Consider: Tab for multiturn toggle

## Stylix Theming
- [ ] Add Stylix NixOS module integration
- [ ] Export theme colors via config file
- [ ] Read Stylix-generated colors in app
- [ ] Apply to egui UI (background, text, accent)

## History Tree View
- [ ] Design: Tree structure similar to pi-coding-agent
  - Each session stored as JSONL
  - Branches for alternative responses
  - Root = first prompt, children = follow-ups
- [ ] Add history button to UI
- [ ] Create tree viewer screen/modal
- [ ] Allow selecting/continuing from history nodes
- [ ] Show timestamps and truncated previews

## Multiturn Toggle (Tab)
- [ ] State: single-turn (default) vs multi-turn
- [ ] Single-turn: Each request is independent
- [ ] Multi-turn: Maintain conversation context
- [ ] Visual indicator of current mode
- [ ] Tab key to toggle

## History Management
- [ ] Default: Temporary session (no persistence)
- [ ] Auto-stash to backup on certain actions
  - Rotation: Keep last 10 backup sessions
  - Location: `~/.local/share/lm-modal/backups/`
- [ ] Backup triggers:
  - Window close with content
  - Explicit save/stash
  - Before clearing
- [ ] Format: JSONL per session
- [ ] Backup rotation: Delete oldest when >10

---

## Implementation Order

1. **Keyboard shortcuts** (foundation for other features)
2. **Multiturn toggle** (needs mode state)
3. **History tree** (needs data structures)
4. **History management** (backup/rotation)
5. **Stylix theming** (visual polish)
6. **Floating window** (nixos-config)