//! UNAPPLIED PROTOTYPE. Not an installed PlutoLib API or native save codec.
//! Proposed opaque values owned by the existing player EffectInstance.
use std::collections::{BTreeMap, BTreeSet};

use super::EffectInstance;

/// Explicit caller-selected prototype limits; no implicit gameplay policy.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_layers: usize,
    pub max_entries_per_layer: usize,
    pub max_key_bytes: usize,
    pub max_value_bytes: usize,
    pub max_id_bytes: usize,
    /// Sum of effect IDs, extension keys and extension bytes across the record.
    /// This is a logical payload bound, not an encoded-size or heap-size claim.
    pub max_total_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidKey,
    DuplicateKey,
    TooManyEntries,
    KeyTooLong,
    ValueTooLong,
    TotalTooLarge,
    EmptyRecord,
    TooManyLayers,
    InvalidId,
    InvalidAmplifier,
    UnresolvedId { layer: usize },
}

/// Unknown namespaces and opaque bytes are retained without interpreting them.
/// No mutable map escape, instance-address key or effect-ID side table exists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Extensions {
    entries: BTreeMap<String, Vec<u8>>,
}

fn key_valid(key: &str) -> bool {
    let Some((namespace, path)) = key.split_once(':') else { return false; };
    let part = |s: &str, slash: bool| !s.is_empty() && s.bytes().all(|b|
        b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
        || (slash && b == b'/'));
    part(namespace, false) && part(path, true)
}

fn add_size(total: &mut usize, amount: usize, limits: Limits) -> Result<(), Error> {
    *total = total.checked_add(amount).ok_or(Error::TotalTooLarge)?;
    if *total > limits.max_total_bytes { return Err(Error::TotalTooLarge); }
    Ok(())
}

fn validate_entries<'a>(
    entries: impl Iterator<Item = (&'a str, &'a [u8])>, limits: Limits,
) -> Result<usize, Error> {
    let mut total = 0;
    for (index, (key, value)) in entries.enumerate() {
        if index >= limits.max_entries_per_layer { return Err(Error::TooManyEntries); }
        if key.len() > limits.max_key_bytes { return Err(Error::KeyTooLong); }
        if !key_valid(key) { return Err(Error::InvalidKey); }
        if value.len() > limits.max_value_bytes { return Err(Error::ValueTooLong); }
        add_size(&mut total, key.len(), limits)?;
        add_size(&mut total, value.len(), limits)?;
    }
    Ok(total)
}

impl Extensions {
    pub fn from_entries(entries: Vec<(String, Vec<u8>)>, limits: Limits) -> Result<Self, Error> {
        validate_entries(entries.iter().map(|(k, v)| (k.as_str(), v.as_slice())), limits)?;
        let mut keys = BTreeSet::new();
        for (key, _) in &entries {
            if !keys.insert(key.as_str()) { return Err(Error::DuplicateKey); }
        }
        Ok(Self { entries: entries.into_iter().collect() })
    }
    pub fn get(&self, key: &str) -> Option<&[u8]> {
        self.entries.get(key).map(Vec::as_slice)
    }
    pub fn entries(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.entries.iter().map(|(key, value)| (key.as_str(), value.as_slice()))
    }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    /// Complete validation precedes mutation; replacing one key is explicit.
    pub fn insert(&mut self, key: String, value: Vec<u8>, limits: Limits) -> Result<(), Error> {
        validate_entries(
            self.entries().filter(|(existing, _)| *existing != key)
                .chain(std::iter::once((key.as_str(), value.as_slice()))), limits,
        )?;
        self.entries.insert(key, value);
        Ok(())
    }
}

/// Proposed owned record, not NBT and not wired into Level.active_effects.
/// Layers are active first, followed by its exact hidden chain in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub layers: Vec<Layer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub id: String,
    pub duration: i32,
    pub amplifier: i32,
    pub ambient: bool,
    pub visible: bool,
    pub extensions: Extensions,
}

fn validate_layer(
    id: &str, amplifier: i32, extensions: &Extensions, total: &mut usize, limits: Limits,
) -> Result<(), Error> {
    if id.is_empty() || id.len() > limits.max_id_bytes { return Err(Error::InvalidId); }
    if !(0..=255).contains(&amplifier) { return Err(Error::InvalidAmplifier); }
    add_size(total, id.len(), limits)?;
    add_size(total, validate_entries(extensions.entries(), limits)?, limits)
}

impl Record {
    /// Validate an owned record without resolving IDs or creating native effects.
    pub fn validate(&self, limits: Limits) -> Result<(), Error> {
        if self.layers.is_empty() { return Err(Error::EmptyRecord); }
        if self.layers.len() > limits.max_layers { return Err(Error::TooManyLayers); }
        let mut total = 0;
        for layer in &self.layers {
            validate_layer(&layer.id, layer.amplifier, &layer.extensions, &mut total, limits)?;
        }
        Ok(())
    }
    /// Validate the whole chain before cloning any extension payload.
    pub fn capture(instance: &EffectInstance, limits: Limits) -> Result<Self, Error> {
        let mut next = Some(instance);
        let mut count = 0;
        let mut total = 0;
        while let Some(effect) = next {
            if count >= limits.max_layers { return Err(Error::TooManyLayers); }
            validate_layer(effect.id, effect.amplifier, &effect.extensions, &mut total, limits)?;
            count += 1;
            next = effect.hidden.as_deref();
        }
        let mut layers = Vec::with_capacity(count);
        let mut next = Some(instance);
        while let Some(effect) = next {
            layers.push(Layer {
                id: effect.id.into(), duration: effect.duration, amplifier: effect.amplifier,
                ambient: effect.ambient, visible: effect.visible, extensions: effect.extensions.clone(),
            });
            next = effect.hidden.as_deref();
        }
        Ok(Self { layers })
    }

    /// The caller supplies the real registry resolver. No leaked strings or
    /// synthetic registration. Validate/resolve every layer before construction.
    /// This does not publish into an actor or replace any existing collection.
    pub fn restore(
        &self, limits: Limits, resolve: impl Fn(&str) -> Option<&'static str>,
    ) -> Result<EffectInstance, Error> {
        self.validate(limits)?;
        let ids: Vec<_> = self.layers.iter().enumerate().map(|(layer, value)|
            resolve(&value.id).ok_or(Error::UnresolvedId { layer })
        ).collect::<Result<_, _>>()?;
        let mut hidden = None;
        for (layer, id) in self.layers.iter().zip(ids).rev() {
            let mut effect = EffectInstance::new(id, layer.duration, layer.amplifier);
            effect.ambient = layer.ambient;
            effect.visible = layer.visible;
            effect.extensions = layer.extensions.clone();
            effect.hidden = hidden;
            hidden = Some(Box::new(effect));
        }
        Ok(*hidden.expect("validated nonempty record"))
    }
}
