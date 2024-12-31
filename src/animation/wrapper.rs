#![allow(dead_code)]
use super::engine::AnimationEngine;
use super::AnimationConfig;
use crate::core::animation::AnimationKind;
use std::ops::Deref;
use std::ops::DerefMut;

#[derive(Debug)]
pub struct AnimationEngineVecOccupiedError {
    existing: AnimationEngine,
    attempted: AnimationEngine,
}

impl AnimationEngineVecOccupiedError {
    fn existing(&self) -> &AnimationEngine {
        &self.existing
    }

    fn attempted(&self) -> &AnimationEngine {
        &self.attempted
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimationEngineVec(Vec<AnimationEngine>);

impl AnimationEngineVec {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn contains_kind(&self, kind: AnimationKind) -> bool {
        self.0.iter().any(|a| a.kind == kind)
    }

    pub fn insert(&mut self, item: AnimationEngine) -> Option<AnimationEngine> {
        for animation in self.0.iter_mut() {
            if animation.kind == item.kind {
                return Some(std::mem::replace(animation, item));
            }
        }
        self.0.push(item);
        None
    }

    pub fn try_insert(
        &mut self,
        item: AnimationEngine,
    ) -> Result<&mut AnimationEngine, AnimationEngineVecOccupiedError> {
        if let Some(pos) = self.0.iter().position(|a| a.kind == item.kind) {
            return Err(AnimationEngineVecOccupiedError {
                existing: self.0[pos].clone(),
                attempted: item,
            });
        }

        self.0.push(item);
        Ok(self.0.last_mut().unwrap())
    }

    pub fn get(&self, k: &AnimationKind) -> Option<&AnimationEngine> {
        self.0.iter().find(|a| a.kind == *k)
    }

    pub fn remove(&mut self, k: &AnimationKind) -> Option<AnimationEngine> {
        if let Some(pos) = self.0.iter().position(|a| a.kind == *k) {
            Some(self.0.remove(pos))
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, k: &AnimationKind) -> Option<&mut AnimationEngine> {
        self.0.iter_mut().find(|a| a.kind == *k)
    }

    pub fn kinds(&self) -> Kinds<AnimationEngine> {
        Kinds(self.0.iter())
    }

    pub fn into_kinds(self) -> IntoKinds<AnimationEngine> {
        IntoKinds(self.0.into_iter())
    }
}

impl Deref for AnimationEngineVec {
    type Target = Vec<AnimationEngine>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AnimationEngineVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for AnimationEngineVec {
    type Item = AnimationEngine;
    type IntoIter = std::vec::IntoIter<AnimationEngine>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a AnimationEngineVec {
    type Item = &'a AnimationEngine;
    type IntoIter = std::slice::Iter<'a, AnimationEngine>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut AnimationEngineVec {
    type Item = &'a mut AnimationEngine;
    type IntoIter = std::slice::IterMut<'a, AnimationEngine>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

pub struct Kinds<'a, V>(std::slice::Iter<'a, V>);

impl<'a> Iterator for Kinds<'a, AnimationEngine> {
    type Item = &'a AnimationKind;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|animation| &animation.kind)
    }
}

pub struct IntoKinds<V>(std::vec::IntoIter<V>);

impl Iterator for IntoKinds<AnimationEngine> {
    type Item = AnimationKind;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|animation| animation.kind)
    }
}

impl TryFrom<Vec<AnimationConfig>> for AnimationEngineVec {
    type Error = anyhow::Error;
    fn try_from(value: Vec<AnimationConfig>) -> Result<AnimationEngineVec, Self::Error> {
        value
            .into_iter()
            .try_fold(AnimationEngineVec::new(), |mut acc, animation_value| {
                let animation = AnimationEngine::try_from(animation_value)?; // Assuming `transform` returns `Result<Animation, anyhow::Error>`
                acc.insert(animation); // Assuming `insert` is defined for AnimationEngineVec
                Ok(acc)
            })
    }
}
