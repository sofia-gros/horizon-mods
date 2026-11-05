# Guide keyboard page navigation

While viewing a marked TConstruct guide, Left Arrow or A turns to the previous
spread, and Right Arrow or D turns to the next. This restores the pinned Mantle
BookScreen navigation keys using the port's existing spread model, cover/last
spread boundaries, link-return reset and page-turn sound.

The new hook intercepts only those four keys at the host `book_input` boundary
for marked non-lectern guide views. Vanilla written/writable books, lecterns,
closed books, unmarked/empty views, Page Up/Down, Escape, text input and unrelated
keys continue through the host unchanged. Existing host keyboard shortcut and
pressed/released handling remains ahead of this boundary. The hook does not
change bindings, add mouse-wheel navigation, retain pages after closing, modify
items or send new network packets. Repeated navigation keys follow the same
bounded spread behavior as the existing guide arrows.

The regression fixture executes the production keyboard and turn helpers with
a small Game/KeyCode fixture plus the production spread code. It covers cover,
odd/even final spreads, repeated turns, history clearing, sound invocation and
excluded screen/key forms. It is not host input-event integration or interactive
GUI/LAN evidence. Full Mixins transformation and host type checking are required
before merge. Actual keyboard/controller input, closing/reopening and host/guest
sessions remain unverified.

Source: [Mantle BookScreen.java at 17c287ad](https://github.com/SlimeKnights/Mantle/blob/17c287ad52f7b10154e07fa7570dc7bccd74ca2b/src/main/java/slimeknights/mantle/client/screen/book/BookScreen.java),
`keyPressed`: Left/A calls `previousPage`, Right/D calls `nextPage`.
Copyright (c) SlimeKnights; MIT license in `../third_party/mantle/LICENSE`.
