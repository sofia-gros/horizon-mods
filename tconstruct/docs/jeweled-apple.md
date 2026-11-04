# Jeweled Apple acquisition

The pinned TConstruct `f4e77d43` table recipe consumes an apple and 200 mB molten diamond and cools for 118 ticks. It is already in the packaged original recipe archive; this change registers its previously missing result and source artwork. Two diamonds each melt into 100 mB at 1450°C. The ordinary progression to suitably hot fuel is separate work: this registration does not supply that fuel or change melting rules.

The apple is always edible, supplies 4 nutrition and 9.6 saturation, and applies Haste I for 1200 ticks and Resistance I for 2400 ticks. Native and desktop resource preparation specify those components. The pinned recipe and lookup test use the source cast, fluid amount, result, consumed-cast flag and cooling time. The result is not a claim that its separate modifier recipe works, or that other missing foods such as cheese work.

Scoped data and desktop resource tests do not prove the live registry, actual casting, eating, effect synchronization, Creative placement, save/reload, custom data packs, integrated host build, or LAN behavior. The isolated Rust lookup harness passed all 74 tests using the source fluid-display module and a copy of the source bucket-resolution function; this is not a full host test.
