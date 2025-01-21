use std::ops::{Deref, DerefMut};

use super::{EffectConfig, engine::EffectEngine};

/// A wrapper around `Vec<EffectEngine>` that provides additional functionality for managing and manipulating a collection of `EffectEngine` instances.
///
/// This struct serves as a container for a list of `EffectEngine` objects. It provides methods to add effects, check for duplicates, and interact with the underlying `Vec<EffectEngine>`.
/// It also implements common Rust traits such as `Deref`, `DerefMut`, and `IntoIterator` to allow for convenient usage like working directly with `Vec` methods.
///
/// The `EffectEngineVec` is particularly useful when managing multiple effects that can be applied to custom window borders, such as glow and shadow effects.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EffectEngineVec(Vec<EffectEngine>);

impl EffectEngineVec {
    /// Creates a new empty `EffectEngineVec`.
    ///
    /// This method initializes an empty container for storing `EffectEngine` objects.
    /// It can be used when you don't have any effects initially but want to add them later.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Creates a new `EffectEngineVec` with the specified initial capacity.
    ///
    /// This method initializes an empty container but allocates enough space to hold `capacity` elements.
    /// This can improve performance when you already know how many effects will be added to the container.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Inserts a new `EffectEngine` into the vector, ensuring no duplicates.
    ///
    /// If an identical effect already exists in the vector, it will return the existing effect.
    /// Otherwise, it will add the new effect to the collection.
    ///
    /// # Arguments
    /// * `item` - The `EffectEngine` to insert into the vector.
    ///
    /// # Returns
    /// * `Some(EffectEngine)` - If the effect was inserted (or already existed), the effect is returned.
    pub fn insert(&mut self, item: EffectEngine) -> Option<EffectEngine> {
        // Check if the effect already exists in the collection
        if let Some(existing) = self.0.iter().find(|effect| **effect == item) {
            return Some(existing.clone());
        }
        // Otherwise, insert the new effect into the collection
        self.0.push(item.clone());
        Some(item)
    }
}

impl Deref for EffectEngineVec {
    type Target = Vec<EffectEngine>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EffectEngineVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for EffectEngineVec {
    type Item = EffectEngine;
    type IntoIter = std::vec::IntoIter<EffectEngine>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a EffectEngineVec {
    type Item = &'a EffectEngine;
    type IntoIter = std::slice::Iter<'a, EffectEngine>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut EffectEngineVec {
    type Item = &'a mut EffectEngine;
    type IntoIter = std::slice::IterMut<'a, EffectEngine>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl TryFrom<Vec<EffectConfig>> for EffectEngineVec {
    type Error = anyhow::Error;
    fn try_from(value: Vec<EffectConfig>) -> Result<EffectEngineVec, Self::Error> {
        value
            .into_iter()
            .try_fold(EffectEngineVec::new(), |mut acc, effect_value| {
                let effect = EffectEngine::try_from(effect_value)?; // Assuming `try_from` returns `Result<EffectEngine, anyhow::Error>`
                acc.insert(effect); // Assuming `insert` is defined for EffectEngineVec
                Ok(acc)
            })
    }
}

