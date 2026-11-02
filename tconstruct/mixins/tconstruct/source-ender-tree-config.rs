// Source-derived Ender tree configurations, normalized for the pinned host.
pub const SMALL: &str = r####"{
  "decorators": [
    {
      "type": "minecraft:attached_to_leaves",
      "block_provider": {
        "type": "minecraft:randomized_int_state_provider",
        "property": "age",
        "source": {
          "id": "tconstruct:ender_slime_sapling",
          "properties": {
            "age": "0",
            "hanging": "true",
            "stage": "0",
            "waterlogged": "false"
          }
        },
        "values": {
          "type": "minecraft:uniform",
          "max_inclusive": 4,
          "min_inclusive": 0
        }
      },
      "directions": [
        "down"
      ],
      "exclusion_radius_xz": 1,
      "exclusion_radius_y": 0,
      "probability": 0.14,
      "required_empty_blocks": 2
    }
  ],
  "foliage_placer": {
    "type": "minecraft:random_spread_foliage_placer",
    "foliage_height": 2,
    "leaf_placement_attempts": 70,
    "offset": 0,
    "radius": 3
  },
  "foliage_provider": {
    "id": "tconstruct:ender_slime_leaves",
    "properties": {
      "distance": "7",
      "persistent": "false",
      "waterlogged": "false"
    }
  },
  "ignore_vines": true,
  "minimum_size": {
    "type": "minecraft:two_layers_feature_size",
    "limit": 2,
    "lower_size": 0,
    "upper_size": 2
  },
  "root_placer": {
    "type": "minecraft:mangrove_root_placer",
    "mangrove_root_placement": {
      "can_grow_through": "#tconstruct:enderbark/roots_can_grow_through",
      "max_root_length": 15,
      "max_root_width": 8,
      "muddy_roots_in": [
        "tconstruct:earth_enderbark_roots",
        "tconstruct:earth_congealed_slime"
      ],
      "muddy_roots_provider": {
        "id": "tconstruct:earth_enderbark_roots"
      },
      "random_skew_chance": 0.2
    },
    "root_provider": {
      "id": "tconstruct:enderbark_roots",
      "properties": {
        "waterlogged": "false"
      }
    },
    "trunk_offset_y": {
      "type": "minecraft:uniform",
      "max_inclusive": 3,
      "min_inclusive": 1
    }
  },
  "trunk_placer": {
    "type": "minecraft:upwards_branching_trunk_placer",
    "base_height": 2,
    "can_grow_through": "#tconstruct:enderbark/logs_can_grow_through",
    "extra_branch_length": {
      "type": "minecraft:uniform",
      "max_inclusive": 1,
      "min_inclusive": 0
    },
    "extra_branch_steps": {
      "type": "minecraft:uniform",
      "max_inclusive": 4,
      "min_inclusive": 1
    },
    "height_rand_a": 1,
    "height_rand_b": 4,
    "place_branch_per_log_probability": 0.5
  },
  "trunk_provider": {
    "id": "tconstruct:enderbark_log",
    "properties": {
      "axis": "y"
    }
  },
  "type": "minecraft:tree",
  "below_trunk_provider": {
    "id": "minecraft:dirt"
  }
}
"####;
pub const TALL: &str = r####"{
  "decorators": [
    {
      "type": "minecraft:attached_to_leaves",
      "block_provider": {
        "type": "minecraft:randomized_int_state_provider",
        "property": "age",
        "source": {
          "id": "tconstruct:ender_slime_sapling",
          "properties": {
            "age": "0",
            "hanging": "true",
            "stage": "0",
            "waterlogged": "false"
          }
        },
        "values": {
          "type": "minecraft:uniform",
          "max_inclusive": 4,
          "min_inclusive": 0
        }
      },
      "directions": [
        "down"
      ],
      "exclusion_radius_xz": 1,
      "exclusion_radius_y": 0,
      "probability": 0.14,
      "required_empty_blocks": 2
    }
  ],
  "foliage_placer": {
    "type": "minecraft:random_spread_foliage_placer",
    "foliage_height": 2,
    "leaf_placement_attempts": 70,
    "offset": 0,
    "radius": 3
  },
  "foliage_provider": {
    "id": "tconstruct:ender_slime_leaves",
    "properties": {
      "distance": "7",
      "persistent": "false",
      "waterlogged": "false"
    }
  },
  "ignore_vines": true,
  "minimum_size": {
    "type": "minecraft:two_layers_feature_size",
    "limit": 3,
    "lower_size": 0,
    "upper_size": 2
  },
  "root_placer": {
    "type": "minecraft:mangrove_root_placer",
    "mangrove_root_placement": {
      "can_grow_through": "#tconstruct:enderbark/roots_can_grow_through",
      "max_root_length": 15,
      "max_root_width": 8,
      "muddy_roots_in": [
        "tconstruct:earth_enderbark_roots",
        "tconstruct:earth_congealed_slime"
      ],
      "muddy_roots_provider": {
        "id": "tconstruct:earth_enderbark_roots"
      },
      "random_skew_chance": 0.2
    },
    "root_provider": {
      "id": "tconstruct:enderbark_roots",
      "properties": {
        "waterlogged": "false"
      }
    },
    "trunk_offset_y": {
      "type": "minecraft:uniform",
      "max_inclusive": 7,
      "min_inclusive": 3
    }
  },
  "trunk_placer": {
    "type": "minecraft:upwards_branching_trunk_placer",
    "base_height": 4,
    "can_grow_through": "#tconstruct:enderbark/logs_can_grow_through",
    "extra_branch_length": {
      "type": "minecraft:uniform",
      "max_inclusive": 1,
      "min_inclusive": 0
    },
    "extra_branch_steps": {
      "type": "minecraft:uniform",
      "max_inclusive": 6,
      "min_inclusive": 1
    },
    "height_rand_a": 1,
    "height_rand_b": 9,
    "place_branch_per_log_probability": 0.5
  },
  "trunk_provider": {
    "id": "tconstruct:enderbark_log",
    "properties": {
      "axis": "y"
    }
  },
  "type": "minecraft:tree",
  "below_trunk_provider": {
    "id": "minecraft:dirt"
  }
}
"####;
