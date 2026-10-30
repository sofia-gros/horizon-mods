# Making parts and tools

Use a Part Builder to make parts, then assemble supported tools at a Tinker Station. The examples below were tested on Linux; the Windows launcher and gameplay remain unverified.

## Browse equipment in Creative

Open your Creative inventory and select the pickaxe tab at the top. Choose an item from the grid to put it on your cursor, then place it in your inventory. Scroll the grid to see more equipment and material variants.

![The Tinkers' Construct Tools tab open in the Creative inventory](pluto-creative-tab.png)

*Select the pickaxe tab, then click an item in the grid. This image is from a Linux test.*

A wood pickaxe obtained here was tested for mining on Linux; a Plate Helmet was obtained and placed in the head slot. Many other tools and armor variants are registered for display, but their material stats and individual actions are still incomplete. Windows remains unverified.

## Make a tool handle

In a creative test world, use the chat to get a Part Builder, patterns, and wood:

```text
/give @s tconstruct:part_builder 1
/give @s tconstruct:pattern 8
/give @s minecraft:oak_log 1
```

Place and open the Part Builder. Put a pattern and the log in the two input slots on the left. Choose **Tool Handle** in the middle, then take the handle from the result slot on the right.

![A Part Builder with a pattern and oak log in its input slots and a tool handle in its result slot](part-builder-source-preview.png)

*Choose Tool Handle in the middle; the handle appears on the right.*

## Assemble a tool

At a Tinker Station, select a supported tool and place its required parts in the input slots. Take the result from the output slot. A pickaxe made with a flint head and wooden parts has been tested through use, repair, head replacement, and saving on Linux.

![A Tinker Station showing a mattock layout and its required parts](station-mattock-lan.png)

*Select a tool to see its part layout. This example shows a mattock in a Linux LAN test.*

Tool-specific behavior, material traits, modifiers, and many recipes are still incomplete. Some test materials shown in development screenshots were supplied by commands or test data and may not yet be obtainable through normal gameplay.

## Repair or replace a part

Open a Tinker Station and place a damaged supported tool and its repair material in the input slots. Take the repaired result. To replace a part, put the tool and a compatible replacement part in the station and take the result. A flint pickaxe repaired with flint and then given a wooden head has been tested on Linux.

![A Tinker Station showing a tool result in a multiplayer test](station-preview-lan.png)

*In this Linux LAN example, the station previews a pickaxe assembled from parts.*

## Use nearby storage

A Part Builder can show a connected container alongside its own slots. You can move items between the displayed storage and the station. A single chest, hopper, dispenser, and double chest were tested on Linux; the exact available operations depend on the container.

![A Part Builder with a hopper displayed beside it](builder-hopper-lan.png)

*The hopper's five slots appear beside the Part Builder inputs in this LAN example.*

## Other equipment

The resource preparation now registers additional source equipment IDs. A Linux test obtained a Broad Axe, Dagger, Plate Helmet, and Skyslime Staff with commands, displayed their icons, and placed the helmet in the head slot. Other acquisition paths, equipment behavior, and worn armor appearance remain incomplete or unverified. Materials, traits, modifiers, and tool-specific actions are still in development.

![A Plate Helmet in the head slot with equipment icons in the hotbar](equipment-registration.png)

*In this Linux test, the Plate Helmet is in the head slot. The axe, dagger, and staff icons are in the hotbar.*

## Use a longbow

Hold the use button to draw a longbow, then release it to fire an arrow. Keep arrows in your inventory. Switching to another item before releasing cancels the shot.

![A longbow drawn with arrows in the hotbar](equipment-longbow.png)

Basic firing and material-specific draw speed were tested on Linux. Material-specific arrow speed and accuracy are not implemented yet.

## Use a crossbow

Hold the use button to load an arrow, release the button, then press it again to fire. Different limb materials change how quickly the crossbow loads. Switching items before it is loaded cancels the charge.

![A crossbow loaded with an arrow](equipment-crossbow.png)

Loading and firing were tested on Linux. Material-specific projectile speed and accuracy are not implemented yet.

## Material traits

The materials used for a tool affect its traits. Wood's Cultivated trait increases material repair; treated wood's Preserved trait increases repair and maximum durability. Steel's Ductile trait increases durability, melee damage, and armor toughness. Flint's Jagged trait increases melee damage as the tool wears; stone's Stonebound trait reduces it as the tool wears. Replace a part at the Tinker Station to change the traits contributed by that part.

![A repaired pickaxe held on the cursor at the Tinker Station](material-traits-repair.png)

These effects were tested on Linux; some station tests used temporary test data. Previously saved tools do not automatically update their maximum durability or stat bonuses: rebuild the tool or replace a part to recalculate them. The mining effects of Jagged and Stonebound, Ductile projectile damage, and other material traits remain incomplete.

![Material variants in the Tinkers Creative tab](trait-stat-creative.png)

The Creative tab offers material variants for inspection. Some materials may not yet be obtainable during normal play.
