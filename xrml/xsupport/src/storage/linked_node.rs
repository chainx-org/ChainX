// Copyright 2018 Chainpool.

use codec::Codec;
use runtime_support::dispatch::Result;
use runtime_support::{StorageMap, StorageValue};

pub trait NodeT {
    type Index: Codec + Clone + Eq + PartialEq + Default;
    fn index(&self) -> Self::Index;
}

#[derive(Decode, Encode, Clone, Default)]
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

pub trait NormalNodeT {
    type NodeType;
    fn data(&self) -> &Self::NodeType;
    fn mut_data(&mut self) -> &mut Self::NodeType;
}

impl<T: NodeT> NormalNodeT for Node<T> {
    type NodeType = Node<T>;
    fn data(&self) -> &Node<T> {
        self
    }
    fn mut_data(&mut self) -> &mut Node<T> {
        self
    }
}

#[derive(Decode, Encode, Eq, PartialEq, Clone, Default)]
pub struct NodeIndex<T: NodeT> {
    index: T::Index,
}

impl<T: NodeT> NodeIndex<T> {
    pub fn index(&self) -> T::Index {
        self.index.clone()
    }
}

pub trait NodeIndexT {
    type IndexType;
    fn data(&self) -> &Self::IndexType;
}

impl<T: NodeT> NodeIndexT for NodeIndex<T> {
    type IndexType = NodeIndex<T>;
    fn data(&self) -> &NodeIndex<T> {
        self
    }
}

pub trait MultiNodeIndexT {
    type KeyType;
    type IndexType;
    fn data(&self) -> &Self::IndexType;
    fn key(&self) -> Self::KeyType;
}

#[derive(Decode, Encode, Eq, PartialEq, Clone, Default)]
pub struct MultiNodeIndex<K, T: NodeT>
where
    K: Codec + Clone + Eq + PartialEq + Default,
{
    index: T::Index,
    multi_key: K,
}

impl<K, T: NodeT> MultiNodeIndex<K, T>
where
    K: Codec + Clone + Eq + PartialEq + Default,
{
    pub fn index(&self) -> T::Index {
        self.index.clone()
    }
}

impl<K, T: NodeT> MultiNodeIndexT for MultiNodeIndex<K, T>
where
    K: Codec + Clone + Eq + PartialEq + Default,
{
    type KeyType = K;
    type IndexType = MultiNodeIndex<K, T>;
    fn data(&self) -> &MultiNodeIndex<K, T> {
        self
    }
    fn key(&self) -> K {
        self.multi_key.clone()
    }
}

pub trait OptionT {
    type OptionType;
    fn data(&self) -> Option<&Self::OptionType>;
    fn mut_data(&mut self) -> Option<&mut Self::OptionType>;
}

impl<T: NodeT> OptionT for Option<Node<T>> {
    type OptionType = Node<T>;
    fn data(&self) -> Option<&Node<T>> {
        match self {
            None => None,
            Some(ref i) => Some(i),
        }
    }

    fn mut_data(&mut self) -> Option<&mut Node<T>> {
        match self {
            None => None,
            Some(ref mut i) => Some(i),
        }
    }
}

impl<T: NodeT> OptionT for Option<NodeIndex<T>> {
    type OptionType = NodeIndex<T>;
    fn data(&self) -> Option<&NodeIndex<T>> {
        match self {
            None => None,
            Some(ref i) => Some(i),
        }
    }

    fn mut_data(&mut self) -> Option<&mut NodeIndex<T>> {
        match self {
            None => None,
            Some(ref mut i) => Some(i),
        }
    }
}

impl<K, T: NodeT> OptionT for Option<MultiNodeIndex<K, T>>
where
    K: Codec + Clone + Eq + PartialEq + Default,
{
    type OptionType = MultiNodeIndex<K, T>;
    fn data(&self) -> Option<&MultiNodeIndex<K, T>> {
        match self {
            None => None,
            Some(ref i) => Some(i),
        }
    }

    fn mut_data(&mut self) -> Option<&mut MultiNodeIndex<K, T>> {
        match self {
            None => None,
            Some(ref mut i) => Some(i),
        }
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

    pub fn add_option_node_before<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header: StorageValue<NodeIndex<T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            OptionT<OptionType = Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.prev {
            Some(p) => {
                C::NodeMap::mutate(p, |prev| {
                    // TODO add Result when substrate update
                    if let Some(ref mut prev_node) = prev.mut_data() {
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

    pub fn add_option_node_after<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            OptionT<OptionType = Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.next {
            Some(n) => {
                C::NodeMap::mutate(n, |next| {
                    // TODO add Result when substrate update
                    if let Some(ref mut next_node) = next.mut_data() {
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

    pub fn remove_option_node<C: LinkedNodeCollection>(&mut self) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header: StorageValue<NodeIndex<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            OptionT<OptionType = Node<T>>,
        <C::Header as StorageValue<NodeIndex<T>>>::Query: OptionT<OptionType = NodeIndex<T>>,
        <C::Tail as StorageValue<NodeIndex<T>>>::Query: OptionT<OptionType = NodeIndex<T>>,
    {
        if self.is_none() {
            let self_index = self.index();
            C::NodeMap::remove(&self_index);
            if let Some(header) = C::Header::get().data() {
                if self_index == header.index {
                    C::Header::kill();
                }
            }

            if let Some(tail) = C::Tail::get().data() {
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
                        if let Some(next_node) = next.mut_data() {
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
                        if let Some(prev_node) = prev.mut_data() {
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
            let prev = self.prev.clone().unwrap_or(Default::default());
            let next = self.next.clone().unwrap_or(Default::default());

            C::NodeMap::mutate(&prev, |prev| {
                // TODO add Result when substrate update
                if let Some(prev_node) = prev.mut_data() {
                    prev_node.next = Some(next.clone());
                    self.prev = None;
                }
            });
            C::NodeMap::mutate(&next, |next| {
                // TODO add Result when substrate update
                if let Some(next_node) = next.mut_data() {
                    next_node.prev = Some(prev.clone());
                    self.next = None;
                }
            });
            C::NodeMap::remove(self.index());
        }
        Ok(())
    }

    pub fn add_node_before<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header: StorageValue<NodeIndex<T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            NormalNodeT<NodeType = Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.prev {
            Some(p) => C::NodeMap::mutate(p, |prev_node| {
                if prev_node.data().is_none() == false {
                    node.prev = Some(prev_node.data().index());
                    node.next = Some(i);
                    prev_node.mut_data().next = Some(node.index());
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

    pub fn add_node_after<C: LinkedNodeCollection>(&mut self, mut node: Node<T>) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            NormalNodeT<NodeType = Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.next {
            Some(n) => C::NodeMap::mutate(n, |next_node| {
                if next_node.data().is_none() == false {
                    node.prev = Some(i);
                    node.next = Some(next_node.data().index());
                    next_node.mut_data().prev = Some(node.index());
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

    pub fn remove_node<C: LinkedNodeCollection>(&mut self) -> Result
    where
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header: StorageValue<NodeIndex<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            NormalNodeT<NodeType = Node<T>>,
        <C::Header as StorageValue<NodeIndex<T>>>::Query: NodeIndexT<IndexType = NodeIndex<T>>,
        <C::Tail as StorageValue<NodeIndex<T>>>::Query: NodeIndexT<IndexType = NodeIndex<T>>,
    {
        if self.is_none() {
            let self_index = self.index();
            let head_index = C::Header::get();
            let tail_index = C::Tail::get();
            C::NodeMap::remove(&self_index);
            if self_index == head_index.data().index && self_index == tail_index.data().index {
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
                    if next_node.data().is_none() == false {
                        next_node.mut_data().prev = None;
                        C::Header::put(NodeIndex::<T> {
                            index: next_node.data().index(),
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
                    if prev_node.data().is_none() == false {
                        prev_node.mut_data().next = None;
                        C::Tail::put(NodeIndex::<T> {
                            index: prev_node.data().index(),
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
            let prev = self.prev.clone().unwrap_or(Default::default());
            let next = self.next.clone().unwrap_or(Default::default());

            C::NodeMap::mutate(&prev, |prev_node| {
                if prev_node.data().is_none() == false {
                    prev_node.mut_data().next = Some(next.clone());
                    self.prev = None;
                }
            });
            C::NodeMap::mutate(&next, |next_node| {
                if next_node.data().is_none() == false {
                    next_node.mut_data().prev = Some(prev.clone());
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
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header: StorageValue<NodeIndex<T>>,
        C::Tail: StorageValue<NodeIndex<T>>,
    {
        if C::Header::exists() == false {
            C::Header::put(NodeIndex::<T> {
                index: self.index(),
            });
        }
        if C::Tail::exists() == false {
            C::Tail::put(NodeIndex::<T> {
                index: self.index(),
            });
        }
        C::NodeMap::insert(self.index(), self);
    }
}

// for multi index
impl<T: NodeT + Codec> Node<T> {
    pub fn init_storage_withkey<C: LinkedNodeCollection, K>(&self, key: K)
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header:
            StorageMap<<MultiNodeIndex<K, T> as MultiNodeIndexT>::KeyType, MultiNodeIndex<K, T>>,
        C::Tail:
            StorageMap<<MultiNodeIndex<K, T> as MultiNodeIndexT>::KeyType, MultiNodeIndex<K, T>>,
    {
        if C::Header::exists(&key) == false {
            C::Header::insert(
                key.clone(),
                MultiNodeIndex::<K, T> {
                    index: self.index(),
                    multi_key: key.clone(),
                },
            );
        }
        if C::Tail::exists(&key) == false {
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

    pub fn add_option_node_before_withkey<C: LinkedNodeCollection, K>(
        &mut self,
        mut node: Node<T>,
        key: K,
    ) -> Result
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header:
            StorageMap<<MultiNodeIndex<K, T> as MultiNodeIndexT>::KeyType, MultiNodeIndex<K, T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            OptionT<OptionType = Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.prev {
            Some(p) => {
                C::NodeMap::mutate(p, |prev| {
                    // TODO add Result when substrate update
                    if let Some(ref mut prev_node) = prev.mut_data() {
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

    pub fn add_option_node_after_withkey<C: LinkedNodeCollection, K>(
        &mut self,
        mut node: Node<T>,
        key: K,
    ) -> Result
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Tail:
            StorageMap<<MultiNodeIndex<K, T> as MultiNodeIndexT>::KeyType, MultiNodeIndex<K, T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            OptionT<OptionType = Node<T>>,
    {
        let i = self.index();
        if i == node.index() {
            return Ok(());
        }
        match &self.next {
            Some(n) => {
                C::NodeMap::mutate(n, |next| {
                    // TODO add Result when substrate update
                    if let Some(ref mut next_node) = next.mut_data() {
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

    pub fn remove_option_node_withkey<C: LinkedNodeCollection, K>(&mut self, key: K) -> Result
    where
        K: Codec + Clone + Eq + PartialEq + Default,
        C::NodeMap: StorageMap<T::Index, Node<T>>,
        C::Header: StorageMap<K, MultiNodeIndex<K, T>>,
        C::Tail: StorageMap<K, MultiNodeIndex<K, T>>,
        <C::NodeMap as StorageMap<<T as NodeT>::Index, Node<T>>>::Query:
            OptionT<OptionType = Node<T>>,
        <C::Header as StorageMap<K, MultiNodeIndex<K, T>>>::Query:
            OptionT<OptionType = MultiNodeIndex<K, T>>,
        <C::Tail as StorageMap<K, MultiNodeIndex<K, T>>>::Query:
            OptionT<OptionType = MultiNodeIndex<K, T>>,
    {
        if self.is_none() {
            let self_index = self.index();
            C::NodeMap::remove(&self_index);
            if let Some(header) = C::Header::get(&key).data() {
                if self_index == header.index {
                    C::Header::remove(&key);
                }
            }

            if let Some(tail) = C::Tail::get(&key).data() {
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
                        if let Some(next_node) = next.mut_data() {
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
                        if let Some(prev_node) = prev.mut_data() {
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
            let prev = self.prev.clone().unwrap_or(Default::default());
            let next = self.next.clone().unwrap_or(Default::default());

            C::NodeMap::mutate(&prev, |prev| {
                // TODO add Result when substrate update
                if let Some(prev_node) = prev.mut_data() {
                    prev_node.next = Some(next.clone());
                    self.prev = None;
                }
            });
            C::NodeMap::mutate(&next, |next| {
                // TODO add Result when substrate update
                if let Some(next_node) = next.mut_data() {
                    next_node.prev = Some(prev.clone());
                    self.next = None;
                }
            });
            C::NodeMap::remove(self.index());
        }
        Ok(())
    }
}
