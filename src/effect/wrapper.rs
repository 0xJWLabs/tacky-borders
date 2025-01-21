use std::ops::{Deref, DerefMut};

use super::{engine::EffectEngine, EffectConfig};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct EffectEngineVec(Vec<EffectEngine>);

impl EffectEngineVec {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn insert(&mut self, item: EffectEngine) -> Option<EffectEngine> {
        if let Some(existing) = self.0.iter().find(|effect| **effect == item) {
            return Some(existing.clone());
        }
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