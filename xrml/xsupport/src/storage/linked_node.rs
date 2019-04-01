// Copyright 2018 Chainpool.

use parity_codec::{Codec, Decode, Encode};
#[cfg(feature = "std")]
use serde_derive::{Deserialize, Serialize};

// Substrate
use primitives::traits::MaybeSerializeDebug;
use support::{dispatch::Result, StorageMap, StorageValue};

pub trait NodeT {
    type Index: Codec + Clone + Eq + PartialEq + Default + MaybeSerializeDebug;
    fn index(&self) -> Self::Index;
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct Node<T: NodeT> {
    prev: Option<T::Index>,
    next: Option<T::Index>,
    pub data: T,
}

impl<T: NodeT> Node<T> {
    pub fn prev(&self) -> Option<T::Index> {
        match &self.prev {
            Some(i) => Some(i.clone()),
            None => None,
        }
    }

    pub fn next(&self) -> Option<T::Index> {
        match &self.next {
            Some(i) => Some(i.clone()),
            None => None,
        }
    }

    pub fn index(&self) -> T::Index {
        self.data.index()
    }

    pub fn is_none(&self) -> bool {
        self.prev.is_none() && self.next.is_none()
    }
}

impl<T> AsRef<Node<T>> for Node<T>
where
    T: NodeT,
{
    fn as_ref(&self) -> &Node<T> {
        self
    }
}

impl<T> AsMut<Node<T>> for Node<T>
where
    T: NodeT,
{
    fn as_mut(&mut self) -> &mut Node<T> {
        self
    }
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct NodeIndex<T: NodeT> {
    index: T::Index,
}

impl<T: NodeT> NodeIndex<T> {
    pub fn index(&self) -> T::Index {
        self.index.clone()
    }
}

impl<T> AsRef<NodeIndex<T>> for NodeIndex<T>
where
    T: NodeT,
{
    fn as_ref(&self) -> &NodeIndex<T> {
        self
    }
}

#[derive(PartialEq, Eq, Clone, Default, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct MultiNodeIndex<K, T>
where
    K: Codec + Clone + Eq + PartialEq + Default,
    T: NodeT,
{
    multi_key: K,
    index: T::Index,
}

impl<K, T> MultiNodeIndex<K, T>
where
    K: Codec + Clone + Eq + PartialEq + Default,
    T: NodeT,
{
    pub fn key(&self) -> K {
        self.multi_key.clone()
    }

    pub fn index(&self) -> T::Index {
        self.index.clone()
    }
}

impl<K, T> AsRef<MultiNodeIndex<K, T>> for MultiNodeIndex<K, T>
where
    K: Codec + Clone + Eq + PartialEq + Default,
    T: NodeT,
{
    fn as_ref(&self) -> &MultiNodeIndex<K, T> {
        self
    }
}

pub trait AsRefAndMutOption<T> {
    fn as_ref(&self) -> Option<&T>;
    fn as_mut(&mut self) -> Option<&mut T>;
}

impl<T> AsRefAndMutOption<Node<T>> for Option<Node<T>>
where
    T: NodeT,
{
    fn as_ref(&self) -> Option<&Node<T>> {
        self.as_ref()
    }

    fn as_mut(&mut self) -> Option<&mut Node<T>> {
        self.as_mut()
    }
}

impl<T> AsRefAndMutOption<NodeIndex<T>> for Option<NodeIndex<T>>
where
    T: NodeT,
{
    fn as_ref(&self) -> Option<&NodeIndex<T>> {
        self.as_ref()
    }

    fn as_mut(&mut self) -> Option<&mut NodeIndex<T>> {
        self.as_mut()
    }
}

impl<K, T> AsRefAndMutOption<MultiNodeIndex<K, T>> for Option<MultiNodeIndex<K, T>>
where
    K: Codec + Clone + Eq + PartialEq + Default,
    T: NodeT,
{
    fn as_ref(&self) -> Option<&MultiNodeIndex<K, T>> {
        self.as_ref()
    }

    fn as_mut(&mut self) -> Option<&mut MultiNodeIndex<K, T>> {
        self.as_mut()
    }
}

pub trait LinkedNodeCollection {
    type Header;
    type NodeMap;
    type Tail;
}

impl<T: NodeT + Codec> Node<T> {
    pub fn new(data: T) -> Node<T> {
        Node::<T> {
            prev: None,
            next: None,
            data,
        }
    }

    pub fn add_option_before<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::Header: StorageValue<NodeIndex<T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRefAndMutOption<Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.prev {
            Some(p) => {
                C::NodeMap::mutate(p, |prev| {
                    // TODO add Result when substrate update
                    if let Some(ref mut prev_node) = prev.as_mut() {
                        node.prev = Some(prev_node.index());
                        node.next = Some(i);
                        prev_node.next = Some(node.index());
                    }
                });
            }
            None => {
                node.prev = None;
                node.next = Some(i);
                C::Header::put(NodeIndex::<T> {
                    index: node.index(),
                });
            }
        }
        if node.is_none() {
            // something err
            return Err("do add for a invalid node");
        }
        self.prev = Some(node.index());
        C::NodeMap::insert(self.index(), self);

        let i = node.index();
        C::NodeMap::insert(i, node);
        Ok(())
    }

    pub fn add_option_after<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRefAndMutOption<Node<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.next {
            Some(n) => {
                C::NodeMap::mutate(n, |next| {
                    // TODO add Result when substrate update
                    if let Some(ref mut next_node) = next.as_mut() {
                        node.prev = Some(i);
                        node.next = Some(next_node.index());
                        next_node.prev = Some(node.index());
                    }
                })
            }
            None => {
                node.prev = Some(i);
                node.next = None;
                C::Tail::put(NodeIndex::<T> {
                    index: node.index(),
                });
            }
        }
        if node.is_none() {
            return Err("do add for a invalid node");
        }
        self.next = Some(node.index());
        C::NodeMap::insert(self.index(), self);
        let i = node.index();
        C::NodeMap::insert(i, node);
        Ok(())
    }

    pub fn remove_option<C: LinkedNodeCollection>(&mut self) -> Result
    where
        C::Header: StorageValue<NodeIndex<T>>,
        <C::Header as StorageValue<NodeIndex<T>>>::Query: AsRefAndMutOption<NodeIndex<T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRefAndMutOption<Node<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
        <C::Tail as StorageValue<NodeIndex<T>>>::Query: AsRefAndMutOption<NodeIndex<T>>,
    {
        if self.is_none() {
            let self_index = self.index();
            C::NodeMap::remove(&self_index);
            if let Some(header) = C::Header::get().as_ref() {
                if self_index == header.index {
                    C::Header::kill();
                }
            }

            if let Some(tail) = C::Tail::get().as_ref() {
                if self_index == tail.index {
                    C::Tail::kill();
                }
            }
            return Ok(());
        }

        if self.prev.is_none() {
            match &self.next {
                Some(next) => {
                    C::NodeMap::mutate(next, |next| {
                        // TODO add Result when substrate update
                        if let Some(next_node) = next.as_mut() {
                            next_node.prev = None;
                            C::Header::put(NodeIndex::<T> {
                                index: next_node.index(),
                            });
                            C::NodeMap::remove(self.index());
                        }
                    })
                }
                None => {
                    // something err
                    return Err("prev is none, next should't be none");
                }
            }
        } else if self.next.is_none() {
            match &self.prev {
                Some(prev) => {
                    C::NodeMap::mutate(prev, |prev| {
                        // TODO add Result when substrate update
                        if let Some(prev_node) = prev.as_mut() {
                            prev_node.next = None;
                            C::Tail::put(NodeIndex::<T> {
                                index: prev_node.index(),
                            });
                            C::NodeMap::remove(self.index());
                        }
                    })
                }
                None => {
                    // something err
                    return Err("next is none, prev should't be none");
                }
            }
        } else {
            let prev = self.prev.clone().unwrap_or_default();
            let next = self.next.clone().unwrap_or_default();

            C::NodeMap::mutate(&prev, |prev| {
                // TODO add Result when substrate update
                if let Some(prev_node) = prev.as_mut() {
                    prev_node.next = Some(next.clone());
                    self.prev = None;
                }
            });
            C::NodeMap::mutate(&next, |next| {
                // TODO add Result when substrate update
                if let Some(next_node) = next.as_mut() {
                    next_node.prev = Some(prev.clone());
                    self.next = None;
                }
            });
            C::NodeMap::remove(self.index());
        }
        Ok(())
    }

    pub fn add_before<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::Header: StorageValue<NodeIndex<T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRef<Node<T>> + AsMut<Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.prev {
            Some(p) => C::NodeMap::mutate(p, |prev_node| {
                if !prev_node.as_ref().is_none() {
                    node.prev = Some(prev_node.as_ref().index());
                    node.next = Some(i);
                    prev_node.as_mut().next = Some(node.index());
                }
            }),
            None => {
                node.prev = None;
                node.next = Some(i);
                C::Header::put(NodeIndex::<T> {
                    index: node.index(),
                });
            }
        }
        if node.is_none() {
            // something err
            return Err("do add for a invalid node");
        }

        self.prev = Some(node.index());
        C::NodeMap::insert(self.index(), self);

        let i = node.index();
        C::NodeMap::insert(i, node);
        Ok(())
    }

    pub fn add_after<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRef<Node<T>> + AsMut<Node<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.next {
            Some(n) => C::NodeMap::mutate(n, |next_node| {
                if !next_node.as_ref().is_none() {
                    node.prev = Some(i);
                    node.next = Some(next_node.as_ref().index());
                    next_node.as_mut().prev = Some(node.index());
                }
            }),
            None => {
                node.prev = Some(i);
                node.next = None;
                C::Tail::put(NodeIndex::<T> {
                    index: node.index(),
                });
            }
        }
        if node.is_none() {
            return Err("do add for a invalid node");
        }
        self.next = Some(node.index());
        C::NodeMap::insert(self.index(), self);
        let i = node.index();
        C::NodeMap::insert(i, node);
        Ok(())
    }

    pub fn remove<C: LinkedNodeCollection>(&mut self) -> Result
    where
        C::Header: StorageValue<NodeIndex<T>>,
        <C::Header as StorageValue<NodeIndex<T>>>::Query: AsRef<NodeIndex<T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRef<Node<T>> + AsMut<Node<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
        <C::Tail as StorageValue<NodeIndex<T>>>::Query: AsRef<NodeIndex<T>>,
    {
        if self.is_none() {
            let self_index = self.index();
            let head_index = C::Header::get();
            let tail_index = C::Tail::get();
            C::NodeMap::remove(&self_index);
            if self_index == head_index.as_ref().index && self_index == tail_index.as_ref().index {
                C::Header::kill();
                C::Tail::kill();
            } else {
                // something err
                return Err("remove the node normally but meet err for header and tail");
            }
            return Ok(());
        }

        if self.prev.is_none() {
            match &self.next {
                Some(next) => C::NodeMap::mutate(next, |next_node| {
                    if !next_node.as_ref().is_none() {
                        next_node.as_mut().prev = None;
                        C::Header::put(NodeIndex::<T> {
                            index: next_node.as_ref().index(),
                        });
                        C::NodeMap::remove(self.index());
                    }
                }),
                None => {
                    // something err
                    return Err("prev is none, next should't be none");
                }
            }
        } else if self.next.is_none() {
            match &self.prev {
                Some(prev) => C::NodeMap::mutate(prev, |prev_node| {
                    if !prev_node.as_ref().is_none() {
                        prev_node.as_mut().next = None;
                        C::Tail::put(NodeIndex::<T> {
                            index: prev_node.as_ref().index(),
                        });
                        C::NodeMap::remove(self.index());
                    }
                }),
                None => {
                    // something err
                    return Err("next is none, prev should't be none");
                }
            }
        } else {
            let prev = self.prev.clone().unwrap_or_default();
            let next = self.next.clone().unwrap_or_default();

            C::NodeMap::mutate(&prev, |prev_node| {
                if !prev_node.as_ref().is_none() {
                    prev_node.as_mut().next = Some(next.clone());
                    self.prev = None;
                }
            });
            C::NodeMap::mutate(&next, |next_node| {
                if !next_node.as_ref().is_none() {
                    next_node.as_mut().prev = Some(prev.clone());
                    self.next = None;
                }
            });
            if self.is_none() {
                C::NodeMap::remove(self.index());
            } else {
                // something err
                return Err("prev or next not exist in the storage yet, do not remove this node, but link maybe has been changed");
            }
        }
        Ok(())
    }

    pub fn init_storage<C: LinkedNodeCollection>(&self)
    where
        C::Header: StorageValue<NodeIndex<T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
    {
        if !C::Header::exists() {
            C::Header::put(NodeIndex::<T> {
                index: self.index(),
            });
        }
        if !C::Tail::exists() {
            C::Tail::put(NodeIndex::<T> {
                index: self.index(),
            });
        }
        C::NodeMap::insert(self.index(), self);
    }
}

// for multi index
impl<T: NodeT + Codec> Node<T> {
    pub fn init_storage_with_key<C: LinkedNodeCollection, K>(&self, key: K)
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::Header: StorageMap<K, MultiNodeIndex<K, T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Tail: StorageMap<K, MultiNodeIndex<K, T>>,
    {
        if !C::Header::exists(&key) {
            C::Header::insert(
                key.clone(),
                MultiNodeIndex::<K, T> {
                    index: self.index(),
                    multi_key: key.clone(),
                },
            );
        }
        if !C::Tail::exists(&key) {
            C::Tail::insert(
                key.clone(),
                MultiNodeIndex::<K, T> {
                    index: self.index(),
                    multi_key: key.clone(),
                },
            );
        }
        C::NodeMap::insert(self.index(), self);
    }

    pub fn add_option_before_with_key<C: LinkedNodeCollection, K>(
        &mut self,
        mut node: Node<T>,
        key: K,
    ) -> Result
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::Header: StorageMap<K, MultiNodeIndex<K, T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRefAndMutOption<Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.prev {
            Some(p) => {
                C::NodeMap::mutate(p, |prev| {
                    // TODO add Result when substrate update
                    if let Some(ref mut prev_node) = prev.as_mut() {
                        node.prev = Some(prev_node.index());
                        node.next = Some(i);
                        prev_node.next = Some(node.index());
                    }
                });
            }
            None => {
                node.prev = None;
                node.next = Some(i);
                C::Header::insert(
                    key.clone(),
                    MultiNodeIndex::<K, T> {
                        index: node.index(),
                        multi_key: key,
                    },
                );
            }
        }
        if node.is_none() {
            // something err
            return Err("do add for a invalid node");
        }
        self.prev = Some(node.index());
        C::NodeMap::insert(self.index(), self);

        let i = node.index();
        C::NodeMap::insert(i, node);
        Ok(())
    }

    pub fn add_option_after_with_key<C: LinkedNodeCollection, K>(
        &mut self,
        mut node: Node<T>,
        key: K,
    ) -> Result
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRefAndMutOption<Node<T>>,
        C::Tail: StorageMap<K, MultiNodeIndex<K, T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.next {
            Some(n) => {
                C::NodeMap::mutate(n, |next| {
                    // TODO add Result when substrate update
                    if let Some(ref mut next_node) = next.as_mut() {
                        node.prev = Some(i);
                        node.next = Some(next_node.index());
                        next_node.prev = Some(node.index());
                    }
                })
            }
            None => {
                node.prev = Some(i);
                node.next = None;
                C::Tail::insert(
                    key.clone(),
                    MultiNodeIndex::<K, T> {
                        index: node.index(),
                        multi_key: key,
                    },
                );
            }
        }
        if node.is_none() {
            return Err("do add for a invalid node");
        }
        self.next = Some(node.index());
        C::NodeMap::insert(self.index(), self);
        let i = node.index();
        C::NodeMap::insert(i, node);
        Ok(())
    }

    pub fn remove_option_with_key<C: LinkedNodeCollection, K>(&mut self, key: K) -> Result
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::Header: StorageMap<K, MultiNodeIndex<K, T>>,
        <C::Header as StorageMap<K, MultiNodeIndex<K, T>>>::Query:
            AsRefAndMutOption<MultiNodeIndex<K, T>>,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        <C::NodeMap as StorageMap<T::Index, Node<T>>>::Query: AsRefAndMutOption<Node<T>>,
        C::Tail: StorageMap<K, MultiNodeIndex<K, T>>,
        <C::Tail as StorageMap<K, MultiNodeIndex<K, T>>>::Query:
            AsRefAndMutOption<MultiNodeIndex<K, T>>,
    {
        if self.is_none() {
            let self_index = self.index();
            C::NodeMap::remove(&self_index);
            if let Some(header) = C::Header::get(&key).as_ref() {
                if self_index == header.index {
                    C::Header::remove(&key);
                }
            }

            if let Some(tail) = C::Tail::get(&key).as_ref() {
                if self_index == tail.index {
                    C::Tail::remove(&key);
                }
            }
            return Ok(());
        }

        if self.prev.is_none() {
            match &self.next {
                Some(next) => {
                    C::NodeMap::mutate(next, |next| {
                        // TODO add Result when substrate update
                        if let Some(next_node) = next.as_mut() {
                            next_node.prev = None;
                            C::Header::insert(
                                key.clone(),
                                MultiNodeIndex::<K, T> {
                                    index: next_node.index(),
                                    multi_key: key,
                                },
                            );
                            C::NodeMap::remove(self.index());
                        }
                    })
                }
                None => {
                    // something err
                    return Err("prev is none, next should't be none");
                }
            }
        } else if self.next.is_none() {
            match &self.prev {
                Some(prev) => {
                    C::NodeMap::mutate(prev, |prev| {
                        // TODO add Result when substrate update
                        if let Some(prev_node) = prev.as_mut() {
                            prev_node.next = None;
                            C::Tail::insert(
                                key.clone(),
                                MultiNodeIndex::<K, T> {
                                    index: prev_node.index(),
                                    multi_key: key,
                                },
                            );
                            C::NodeMap::remove(self.index());
                        }
                    })
                }
                None => {
                    // something err
                    return Err("next is none, prev should't be none");
                }
            }
        } else {
            let prev = self.prev.clone().unwrap_or_default();
            let next = self.next.clone().unwrap_or_default();

            C::NodeMap::mutate(&prev, |prev| {
                // TODO add Result when substrate update
                if let Some(prev_node) = prev.as_mut() {
                    prev_node.next = Some(next.clone());
                    self.prev = None;
                }
            });
            C::NodeMap::mutate(&next, |next| {
                // TODO add Result when substrate update
                if let Some(next_node) = next.as_mut() {
                    next_node.prev = Some(prev.clone());
                    self.next = None;
                }
            });
            C::NodeMap::remove(self.index());
        }
        Ok(())
    }
}
