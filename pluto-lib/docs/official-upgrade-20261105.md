# Latest official recovery baseline

This is a development candidate, not a compatibility or release claim.

At the 2026-11-05 15:40 UTC integration cutoff, the official main branches were:

- Horizon: `624c4f6265c02296fbe8a80f029dd0d987fb94c0`.
- Horizon Mixins: `e8cd0aa38cffef49cb2e088ef716baa107148c02`.

TConstruct and every component of the single PlutoLib native bundle target this
same pair. Production fixture runners require a clean checkout of this exact
Horizon commit. Historical demos and the standalone registration crate's older
parser pin are separate targets and are not silently upgraded.

PlutoLib follows official Horizon and Mixins. Shared host adapters and reusable
services belong in PlutoLib; TConstruct consumes their APIs and supplies its
own content and gameplay policy. Neither an engine fork nor changes to the
official source repository are part of this migration. Mixins transforms an
exported copy of official source.

## Validation status

The native package tests and repository release-metadata tests pass with these
pins. The latest Mixins CLI builds from the exact official source. These checks
do not prove the combined TConstruct/PlutoLib candidate can plan, apply, compile
or run on the new Horizon target. Those are independent integration gates.

The preceding merged preservation batch, PR #927, was checked and built against
official Horizon `0c7b5b61a5e746d621afffba919b12aeccf9c0fa` and Mixins
`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. Its successful results remain evidence
for that older pair only. They do not validate this upgrade.

## Promotion gates

1. Keep the generic API implementation independent of TConstruct. Check at
   least one independent consumer and the no-callback vanilla behavior.
2. Validate the actual packaged bundles together with official Mixins. Check
   source references, unique IDs, dependency direction and one registry instance.
3. Check and build the generated host; run the relevant consolidated functional
   fixtures without suppressing unrelated or baseline failures.
4. Verify affected player paths, save/reload and representative LAN behavior.
   Report unavailable or unrun checks rather than treating them as passed.
5. Re-read both official main heads before promotion. If either moved, record
   the new target and repeat affected gates; do not mix evidence from different
   commits or silently label an older target as latest.

Keeping host adaptation in PlutoLib reduces changes needed in consumers. It
does not guarantee that every future upstream change can be absorbed without
an API change, new capability or consumer migration.
