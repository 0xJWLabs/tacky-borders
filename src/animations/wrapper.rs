#![allow(dead_code)]
use super::animation::{Animation, AnimationConfig, AnimationKind};
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
pub struct AnimationsVecOccupiedError {
    existing: Animation,
    attempted: Animation,
}

impl AnimationsVecOccupiedError {
    fn existing(&self) -> &Animation {
        &self.existing
    }

    fn attempted(&self) -> &Animation {
        &self.attempted
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AnimationsVec(Vec<Animation>);

impl AnimationsVec {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn contains_kind(&self, kind: AnimationKind) -> bool {
        self.0.iter().any(|a| a.kind == kind)
    }

    pub fn insert(&mut self, item: Animation) -> Option<Animation> {
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
        item: Animation,
    ) -> Result<&mut Animation, AnimationsVecOccupiedError> {
        if let Some(pos) = self.0.iter().position(|a| a.kind == item.kind) {
            return Err(AnimationsVecOccupiedError {
                existing: self.0[pos].clone(),
                attempted: item,
            });
        }

        self.0.push(item);
        Ok(self.0.last_mut().unwrap())
    }

    pub fn get(&self, k: &AnimationKind) -> Option<&Animation> {
        self.0.iter().find(|a| a.kind == *k)
    }

    pub fn remove(&mut self, k: &AnimationKind) -> Option<Animation> {
        if let Some(pos) = self.0.iter().position(|a| a.kind == *k) {
            Some(self.0.remove(pos))
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, k: &AnimationKind) -> Option<&mut Animation> {
        self.0.iter_mut().find(|a| a.kind == *k)
    }

    pub fn kinds(&self) -> Kinds<Animation> {
        Kinds(self.0.iter())
    }

    pub fn into_kinds(self) -> IntoKinds<Animation> {
        IntoKinds(self.0.into_iter())
    }
}

impl Deref for AnimationsVec {
    type Target = Vec<Animation>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AnimationsVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for AnimationsVec {
    type Item = Animation;
    type IntoIter = std::vec::IntoIter<Animation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a AnimationsVec {
    type Item = &'a Animation;
    type IntoIter = std::slice::Iter<'a, Animation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut AnimationsVec {
    type Item = &'a mut Animation;
    type IntoIter = std::slice::IterMut<'a, Animation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

pub struct Kinds<'a, V>(std::slice::Iter<'a, V>);

impl<'a> Iterator for Kinds<'a, Animation> {
    type Item = &'a AnimationKind;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|animation| &animation.kind)
    }
}

pub struct IntoKinds<V>(std::vec::IntoIter<V>);

impl Iterator for IntoKinds<Animation> {
    type Item = AnimationKind;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|animation| animation.kind)
    }
}

impl TryFrom<Vec<AnimationConfig>> for AnimationsVec {
    type Error = anyhow::Error;
    fn try_from(value: Vec<AnimationConfig>) -> Result<AnimationsVec, Self::Error> {
        value
            .into_iter()
            .try_fold(AnimationsVec::new(), |mut acc, animation_value| {
                let animation = Animation::try_from(animation_value)?; // Assuming `transform` returns `Result<Animation, anyhow::Error>`
                acc.insert(animation); // Assuming `insert` is defined for AnimationsVec
                Ok(acc)
            })
    }
}
