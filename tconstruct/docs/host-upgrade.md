# Horizon host update (draft)

This work targets official Horizon `e12185820a6bc85dd5ed04adc7b8b7a4bdc16102` and Horizon Mixins `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. It adapts the existing TConstruct integrations before the slime-island draft is based on it. This is a compatibility work in progress, not a validated build or a claim that slime islands generate.

The new host supports content block `state_properties` and `default_state`, which may represent the source vine stages. Its world fluid kind still contains only empty, water and lava families; the original slime world fluids remain unresolved. Do not enable incomplete island generation or replace slime fluid with water.

Plan, type checking and representative regression results will be recorded here once run. Existing crafting, GUI, save/reload, LAN, equipment and casting paths all need revalidation after adaptation.
