// Copyright © 2025
// Author: Nocthir <nocthir@proton.me>
// SPDX-License-Identifier: MIT or Apache-2.0

//! Geosets: appearance variant runtime.
//!
//! Many character / equipment models embed multiple alternative mesh fragments
//! (hair styles, cape shapes, facial hair groups, robe lower halves, optional
//! attachments, etc.). The underlying binary format encodes these as *geoset
//! ids* like `1507`: the high‑order two decimal digits (`15`) imply a coarse
//! category, the trailing digits (`07`) are a style / variant index.
//!
//! This module turns those raw ids into a small ECS driven system:
//! * Child mesh entities carry a [`Geoset`] component (raw id classified into a
//!   [`GeosetType`]).
//! * A model root receives a lazily constructed [`GeosetCatalog`] (one‑time
//!   variant discovery) plus a mutable [`GeosetSelection`] the first frame it
//!   appears (`build_geoset_catalog_system`).
//! * Visibility changes only happen when selection changes
//!   (`apply_geoset_selection_system`), avoiding per‑frame recomputation.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};

use crate::assets::model::Model;

/// High-level grouping for character model component / equipment visibility.
///
/// This enum encodes commonly observed geoset category indices used by a
/// character model format. They are typically identified by the high-order
/// digits of the geoset id (e.g. `15xx` for a cloak / cape group). The trailing
/// digits select a style / variation within that category (e.g. different
/// silhouette shapes, beard styles, etc). The definitions are intentionally
/// coarse – they do not enumerate every style id, only the parent category. Use
/// the raw `GeosetType.id` (or a future helper) if exact variant selection is
/// required.
///
/// Sources: community reverse engineering of the binary model format and
/// inspection of shipped assets. This is best-effort and may evolve.
/// https://wowdev.wiki/Character_Customization#Geosets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GeosetType {
    /// Base skin / body root (id 0000). Often always present.
    SkinBase,
    /// Hair / head styles (`00**` except 0000) – variants encode hairstyle meshes.
    Hair,
    /// Facial hair group 1 / "Facial1" (`01**`) – usually beards (style 1..=8 typical).
    Facial1,
    /// Facial hair group 2 / sideburns / alt moustache (`02**`). Style 1 usually none.
    Facial2,
    /// Facial hair group 3 / moustache / alt sideburns (`03**`). Style 1 usually none.
    Facial3,
    /// Gloves (`04**`) – hand coverings, 1..=4 styles.
    Gloves,
    /// Boots / footwear (`05**`) – 1..=5 styles (shape / height).
    Boots,
    /// Shirt tail (race / gender specific lower garment extension) (`06**`). Rare / optional.
    ShirtTail,
    /// Ears (`07**`) – style 1 none / hidden, style 2 visible ears (race dependent).
    Ears,
    /// Wristbands / sleeves (`08**`) – 1 none, 2 normal, 3 ruffled (where supported).
    Wristbands,
    /// Legs armor pads / cuffs (`09**`) – 1 none, 2 long, 3 short variations.
    Legs,
    /// Shirt doublet / upper chest overlay (`10**`) – 1 none, 2 obscure/unused variant.
    ShirtDoublet,
    /// Pant doublet / lower garment overlay (`11**`) – styles include skirt / armored.
    PantDoublet,
    /// Tabard (`12**`) – 1 none, 2 tabard mesh.
    Tabard,
    /// Robe trousers split / dress (`13**`) – 1 legs (pants), 2 dress (robe lower).
    Robe,
    /// Loincloth / lower flap accessory (`14**`).
    Loincloth,
    /// Cloaks / capes (`15**`) – multiple silhouette styles (1..=10 common).
    Cape,
    /// Facial jewelry / adornments (`16**`) – nose rings, earrings, chin pieces, etc.
    FacialJewelry,
    /// Eye glow / special eye effects (`17**`) – 1 none, 2 primary glow, 3 alternate glow.
    EyeEffects,
    /// Belt / belt pack (`18**`) – includes bulky / monk specific variations.
    Belt,
    /// Skin extras: bones / tail / additional appendages (`19**`). Implementation dependent.
    SkinBoneTail,
    /// Toes / feet detail (`20**`) – 1 none, 2 feet (race dependent visibility control).
    Toes,
    /// Skull (additional overlay / effect) (`21**`). Rare usage.
    Skull,
    /// Torso armored overlay (`22**`) – 1 regular, 2 armored chest plating.
    Torso,
    /// Hands attachments (special hand overlays / alternative meshes) (`23**`).
    HandsAttachments,
    /// Head attachments (horns, antlers, crests, etc.) (`24**`).
    HeadAttachments,
    /// Facewear (blindfolds, runes, etc.) (`25**`).
    Facewear,
    /// Shoulders effect / attachment geosets (`26**`).
    Shoulders,
    /// Helm (object component models / extra helm geometry) (`27**`).
    Helm,
    /// Upper arm overlays / attachments (`28**`).
    ArmUpper,
    /// Unknown / not yet classified category.
    Unknown,
}

impl GeosetType {
    pub fn as_str(self) -> &'static str {
        match self {
            GeosetType::SkinBase => "SkinBase",
            GeosetType::Hair => "Hair",
            GeosetType::Facial1 => "Facial1",
            GeosetType::Facial2 => "Facial2",
            GeosetType::Facial3 => "Facial3",
            GeosetType::Gloves => "Gloves",
            GeosetType::Boots => "Boots",
            GeosetType::ShirtTail => "ShirtTail",
            GeosetType::Ears => "Ears",
            GeosetType::Wristbands => "Wristbands",
            GeosetType::Legs => "Legs",
            GeosetType::ShirtDoublet => "ShirtDoublet",
            GeosetType::PantDoublet => "PantDoublet",
            GeosetType::Tabard => "Tabard",
            GeosetType::Robe => "Robe",
            GeosetType::Loincloth => "Loincloth",
            GeosetType::Cape => "Cape",
            GeosetType::FacialJewelry => "FacialJewelry",
            GeosetType::EyeEffects => "EyeEffects",
            GeosetType::Belt => "Belt",
            GeosetType::SkinBoneTail => "SkinBoneTail",
            GeosetType::Toes => "Toes",
            GeosetType::Skull => "Skull",
            GeosetType::Torso => "Torso",
            GeosetType::HandsAttachments => "HandsAttachments",
            GeosetType::HeadAttachments => "HeadAttachments",
            GeosetType::Facewear => "Facewear",
            GeosetType::Shoulders => "Shoulders",
            GeosetType::Helm => "Helm",
            GeosetType::ArmUpper => "ArmUpper",
            GeosetType::Unknown => "Unknown",
        }
    }

    /// Which categories enforce mutual exclusion (only one variant visible).
    pub fn is_exclusive(self) -> bool {
        matches!(
            self,
            GeosetType::SkinBase
                | GeosetType::Boots
                | GeosetType::Hair
                | GeosetType::Facial1
                | GeosetType::Facial2
                | GeosetType::Facial3
                | GeosetType::Ears
                | GeosetType::Cape
                | GeosetType::Gloves
                | GeosetType::Robe
                | GeosetType::Tabard
                | GeosetType::Helm
                | GeosetType::Shoulders
                | GeosetType::Facewear
                | GeosetType::HeadAttachments
                | GeosetType::EyeEffects
        )
    }

    /// Which exclusive categories auto‑select their first discovered variant at catalog build.
    /// Tabard intentionally returns false so it starts hidden (can be enabled manually or by data).
    pub fn default_select(self) -> bool {
        match self {
        GeosetType::SkinBase |
        GeosetType::Hair |
        GeosetType::Facial1 |
        GeosetType::Facial2 |
        GeosetType::Facial3 |
        GeosetType::Cape | // keep cape visible initially; adjust if desired
        GeosetType::Robe | // robe lower part normally chosen with outfit
        GeosetType::Helm | // depends on model; can be toggled off via selection later
        GeosetType::Shoulders |
        GeosetType::Facewear |
        GeosetType::HeadAttachments |
        GeosetType::Gloves |
        GeosetType::Boots |
        GeosetType::EyeEffects => true,
        GeosetType::Tabard => false, // do not show by default
        _ => false,
    }
    }

    /// Categories whose every variant remains visible simultaneously (currently only `Ears`).
    pub fn all_variants_always_visible(self) -> bool {
        matches!(self, GeosetType::Ears)
    }
}

impl std::fmt::Display for GeosetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<u16> for GeosetType {
    /// Classify a raw geoset id (e.g. `1507`) into a high‑level category. Falls back to `Unknown`.
    fn from(id: u16) -> Self {
        // High-order two digits (base 10) normally define the category, except 0000.
        if id == 0 {
            return GeosetType::SkinBase;
        }
        let group = id / 100; // e.g. 15 for 1507
        match group {
            0 => GeosetType::Hair, // 00** excluding 0000
            1 => GeosetType::Facial1,
            2 => GeosetType::Facial2,
            3 => GeosetType::Facial3,
            4 => GeosetType::Gloves,
            5 => GeosetType::Boots,
            6 => GeosetType::ShirtTail,
            7 => GeosetType::Ears,
            8 => GeosetType::Wristbands,
            9 => GeosetType::Legs,
            10 => GeosetType::ShirtDoublet,
            11 => GeosetType::PantDoublet,
            12 => GeosetType::Tabard,
            13 => GeosetType::Robe,
            14 => GeosetType::Loincloth,
            15 => GeosetType::Cape,
            16 => GeosetType::FacialJewelry,
            17 => GeosetType::EyeEffects,
            18 => GeosetType::Belt,
            19 => GeosetType::SkinBoneTail,
            20 => GeosetType::Toes,
            21 => GeosetType::Skull,
            22 => GeosetType::Torso,
            23 => GeosetType::HandsAttachments,
            24 => GeosetType::HeadAttachments,
            25 => GeosetType::Facewear,
            26 => GeosetType::Shoulders,
            27 => GeosetType::Helm,
            28 => GeosetType::ArmUpper,
            _ => GeosetType::Unknown,
        }
    }
}

/// Attached to each selectable mesh fragment (variant) entity.
#[derive(Component, Debug, Copy, Clone, Reflect)]
#[reflect(Component)]
pub struct Geoset {
    pub raw_id: u16,          // original encoded id
    pub category: GeosetType, // derived high‑level category
    pub variant: u16,         // trailing two digits (00..99) – style index
}

impl Geoset {
    pub fn new(id: u16) -> Self {
        Self {
            raw_id: id,
            category: GeosetType::from(id),
            variant: id % 100,
        }
    }
}

/// One‑time discovered set of variants per category for a single model root.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct GeosetCatalog {
    pub categories: HashMap<GeosetType, Vec<u16>>, // sorted ascending variants
}

impl GeosetCatalog {
    pub fn variants(&self, category: GeosetType) -> Option<&[u16]> {
        self.categories.get(&category).map(|v| v.as_slice())
    }
}

/// Mutable selection state
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct GeosetSelection {
    /// Exclusive categories map to a single variant
    exclusive: HashMap<GeosetType, u16>,
    /// Additive categories store enabled variants
    additive: HashMap<GeosetType, HashSet<u16>>,
}

impl GeosetSelection {
    pub fn set_exclusive(&mut self, category: GeosetType, variant: u16) {
        self.exclusive.insert(category, variant);
    }

    pub fn clear_exclusive(&mut self, category: GeosetType) {
        self.exclusive.remove(&category);
    }

    pub fn toggle_additive(&mut self, category: GeosetType, variant: u16) {
        let set = self.additive.entry(category).or_default();
        if !set.insert(variant) {
            set.remove(&variant);
        }
    }

    pub fn selected_exclusive(&self, category: GeosetType) -> Option<u16> {
        self.exclusive.get(&category).copied()
    }

    pub fn is_additive_enabled(&self, category: GeosetType, variant: u16) -> bool {
        self.additive
            .get(&category)
            .map(|s| s.contains(&variant))
            .unwrap_or(false)
    }

    pub fn cycle(&mut self, category: GeosetType, catalog: &GeosetCatalog) {
        if let Some(list) = catalog.variants(category) {
            if list.is_empty() {
                return;
            }
            let current = self.selected_exclusive(category).unwrap_or(list[0]);
            let idx = list.iter().position(|v| *v == current).unwrap_or(0);
            let next = list[(idx + 1) % list.len()];
            self.set_exclusive(category, next);
        }
    }
}

/// Fire once when a Model appears that doesn't yet have a catalog
type ModelQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static Children), (Added<Model>, Without<GeosetCatalog>)>;

/// Build a catalog + initial selection for any newly added model root.
pub fn build_geoset_catalog_system(
    mut commands: Commands,
    roots: ModelQuery,
    variants: Query<&Geoset>,
) {
    for (root, children) in &roots {
        let mut map: HashMap<GeosetType, HashSet<u16>> = HashMap::default();
        for child in children.iter() {
            if let Ok(var) = variants.get(child) {
                map.entry(var.category).or_default().insert(var.variant);
            }
        }
        let mut catalog = GeosetCatalog::default();
        for (cat, set) in map.into_iter() {
            let mut v: Vec<u16> = set.into_iter().collect();
            v.sort_unstable();
            catalog.categories.insert(cat, v);
        }
        // Initialize selection defaults (pick first variant for exclusive categories)
        let mut selection = GeosetSelection::default();
        for (cat, variants) in &catalog.categories {
            if cat.is_exclusive()
                && cat.default_select()
                && let Some(first) = variants.first()
            {
                selection.set_exclusive(*cat, *first);
            }
        }
        commands.entity(root).insert((selection, catalog));
    }
}

/// Apply selection state to child variant entity `Visibility` components.
pub fn apply_geoset_selection_system(
    selections: Query<(&GeosetSelection, &Children), Changed<GeosetSelection>>,
    mut variant_query: Query<(&Geoset, &mut Visibility)>,
) {
    for (selection, children) in &selections {
        for child in children.iter() {
            if let Ok((variant, mut vis)) = variant_query.get_mut(child) {
                let visible = if variant.category == GeosetType::SkinBase
                    || variant.category.all_variants_always_visible()
                {
                    true
                } else if variant.category.is_exclusive() {
                    selection.selected_exclusive(variant.category) == Some(variant.variant)
                } else {
                    selection.is_additive_enabled(variant.category, variant.variant)
                };
                *vis = if visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

/// Debug convenience: press `C` to cycle cape variants (temporary until input mapping exists).
pub fn debug_cycle_cape_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut roots: Query<(&mut GeosetSelection, &GeosetCatalog)>,
) {
    if !keys.just_pressed(KeyCode::KeyC) {
        return;
    }
    for (mut sel, catalog) in &mut roots {
        sel.cycle(GeosetType::Cape, catalog);
    }
}

/// Plugin to register reflection and systems
pub struct GeosetRuntimePlugin;

impl Plugin for GeosetRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Geoset>()
            .register_type::<GeosetCatalog>()
            .register_type::<GeosetSelection>()
            .add_systems(
                Update,
                (
                    build_geoset_catalog_system,
                    apply_geoset_selection_system,
                    debug_cycle_cape_system,
                ),
            );
    }
}
