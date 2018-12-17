// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate parity_codec_derive;
extern crate parity_codec as codec;

// for substrate
extern crate substrate_primitives;

// Assert macros used in tests.
extern crate sr_std;

// for substrate runtime
extern crate sr_std as rstd;

extern crate sr_io as runtime_io;
extern crate sr_primitives as primitives;

// for substrate runtime module lib
#[macro_use]
extern crate srml_support as support;
extern crate srml_system as system;
extern crate srml_balances as balances;

// for chainx runtime module lib
extern crate cxrml_support as cxsupport;

use rstd::prelude::*;
use codec::Codec;
use support::StorageValue;
use support::dispatch::Result;
use primitives::traits::{SimpleArithmetic, As};

//use system::ensure_signed;

use cxsupport::storage::linked_node::{Node, NodeT, NodeIndex, LinkedNodeCollection,
                                      MultiNodeIndex, MultiNodeIndexT,
};

use cxsupport::storage::btree_map::CodecBTreeMap;

pub trait Trait: balances::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

decl_event!(
    pub enum Event<T> where B = <T as system::Trait>::AccountId {
        // Just a normal `enum`, here's a dummy event to ensure it compiles.
        /// Dummy event, just here so there's a generic type that's used.
        Test(B),
    }
);

/// for linked node data, must be Decode, Encode, and Default, the Index(this example is AccountId)
/// must be Ord and Clone
#[derive(Decode, Encode, Eq, PartialEq, Clone, Default)]
pub struct Order<AccountId, Balance>
    where AccountId: Codec + Clone + Ord + Default, Balance: Codec + SimpleArithmetic + Ord + As<u64> + Clone + Copy + Default
{
    pub id: AccountId,
    pub data: Balance,
}

/// 1. impl NodeT for this Data struct, and point the which is index
impl<AccountId, Balance> NodeT for Order<AccountId, Balance>
    where AccountId: Codec + Clone + Ord + Default, Balance: Codec + SimpleArithmetic + Ord + As<u64> + Clone + Copy + Default
{
    type Index = AccountId;

    fn index(&self) -> AccountId {
        self.id.clone()
    }
}

/// 2. create a Phantom struct and let LinkedNodeCollection impl it, notice this LinkedNodeCollection's associate type
/// linkedNode Collection Trait Phantom impl
struct LinkedNodes<T: Trait>(support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedNodes<T> {
    type Header = NodeHeader<T>;
    type NodeMap = NodeMap<T>;
    type Tail = NodeTail<T>;
}

/// 2.2 create a Phantom struct and let LinkedNodeCollection impl it, notice this LinkedNodeCollection's associate type
/// if use Option Node, all type must be Option mode
#[allow(unused)]
struct LinkedOptionNodes<T: Trait> (support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedOptionNodes<T> {
    type Header = OpNodeHeader<T>;
    type NodeMap = OpNodeMap<T>;
    type Tail = OpNodeTail<T>;
}

#[allow(unused)]
struct LinkedOptionMultiKey<T: Trait> (support::storage::generator::PhantomData<T>);

impl<T: Trait> LinkedNodeCollection for LinkedOptionMultiKey<T> {
    type Header = MultiHeader<T>;
    type NodeMap = OpNodeMap2<T>;
    type Tail = MultiTail<T>;
}

/// 3 create Node elements,
/// 3.2 create Option Node elements
decl_storage! {
    trait Store for Module<T: Trait> as CXExample {
        pub Fee get(fee) config(): T::Balance;
        /// btreemap
        pub Map get(map): CodecBTreeMap<T::AccountId, T::Balance>;

        // no Option node group
        /// linked node header, must use `NodeIndex` to wrap the data struct, and the type must be `StorageValue`
        pub NodeHeader get(node_header): NodeIndex<Order<T::AccountId, T::Balance>>;
        /// linked node tail, must use `NodeIndex` to wrap the data struct, and the type must be `StorageValue`
        pub NodeTail get(node_tail): NodeIndex<Order<T::AccountId, T::Balance>>;
        /// linked node collection, must use `Node` to wrap the data struct, and the type must be `StorageMap`, the key must be the index for the data struct
        pub NodeMap get(node_map): map T::AccountId => Node<Order<T::AccountId, T::Balance>>;

        // Option node group
        /// linked node header, must use `NodeIndex` to wrap the data struct, and the type must be `StorageValue`, must be wrapped by Option
        pub OpNodeHeader get(op_node_header): Option<NodeIndex<Order<T::AccountId, T::Balance>>>;
        /// linked node tail, must use `NodeIndex` to wrap the data struct, and the type must be `StorageValue`, must be wrapped by Option
        pub OpNodeTail get(op_node_tail): Option<NodeIndex<Order<T::AccountId, T::Balance>>>;
        /// linked node collection, must use `Node` to wrap the data struct, and the type must be `StorageMap`, the key must be the index for the data struct,
        /// must be wrapped by Option
        pub OpNodeMap get(op_node_map): map T::AccountId => Option<Node<Order<T::AccountId, T::Balance>>>;

        pub MultiHeader get(multi_header): map <MultiNodeIndex<u32, Order<T::AccountId, T::Balance>> as MultiNodeIndexT>::KeyType => Option<MultiNodeIndex<u32, Order<T::AccountId, T::Balance>>>;
        pub MultiTail get(multi_tail): map <MultiNodeIndex<u32, Order<T::AccountId, T::Balance>> as MultiNodeIndexT>::KeyType => Option<MultiNodeIndex<u32, Order<T::AccountId, T::Balance>>>;
        pub OpNodeMap2 get(op_node_map2): map T::AccountId => Option<Node<Order<T::AccountId, T::Balance>>>;
    }
}

#[allow(unused)]
type NodeOrder<T> = Node<Order<<T as system::Trait>::AccountId, <T as balances::Trait>::Balance>>;

impl<T: Trait> Module<T> {
    /// example for linkedNode, when receive a data,
    /// 1. use Node to wrap it,
    /// 2. call `init_storage` to set it to storage, use the prev Phantom struct to fill the template
    /// 3. lookup the node by youself
    /// 4. call `add_node_before`, `add_node_after`, `remove_node` to modify the linked node collection
    #[allow(unused)]
    fn for_linkednode_example(node: NodeOrder<T>) -> Result {
        let mut n: NodeOrder<T> = Node::new(Default::default());
        n.init_storage::<LinkedNodes<T>>();
        n.add_node_before::<LinkedNodes<T>>(node)?;
        // n.add_node_after::<NodeMap<T>, NodeTail<T>>(node);
        // n.remove_node::<NodeMap<T>, NodeHeader<T>, NodeTail<T>>();

        // option type
        // n.add_option_node_before::<LinkedOptionNodes<T>>(node);
        Ok(())
    }

    #[allow(unused)]
    fn for_linkednode_multikey_example(node: NodeOrder<T>) -> Result {
        // option type
        let mut n: NodeOrder<T> = Node::new(Default::default());
        n.init_storage_withkey::<LinkedOptionMultiKey<T>, u32>(1);
        n.add_option_node_before_withkey::<LinkedOptionMultiKey<T>, u32>(node, 1)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use runtime_io::with_externalities;
    use substrate_primitives::{H256, Blake2Hasher};
    use primitives::BuildStorage;
    use primitives::traits::BlakeTwo256;
    use primitives::testing::{Digest, DigestItem, Header};

    use support::{StorageValue, StorageMap};
    //    use support::generator::StorageMap;
    use cxsupport::storage::linked_node::Node;

    impl_outer_origin! {
        pub enum Origin for Test {}
    }
    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;

    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }

    impl balances::Trait for Test {
        type Balance = u64;
        type AccountIndex = u64;
        type OnFreeBalanceZero = ();
        type EnsureAccountLiquid = ();
        type Event = ();
    }

    impl Trait for Test {
        type Event = ();
    }

    pub fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let mut r = system::GenesisConfig::<Test>::default().build_storage().unwrap();
        r.extend(balances::GenesisConfig::<Test> {
            balances: vec![(1, 1000), (2, 510)],
            transaction_base_fee: 0,
            transaction_byte_fee: 0,
            existential_deposit: 500,
            transfer_fee: 0,
            creation_fee: 0,
            reclaim_rebate: 0,
        }.build_storage().unwrap());
        r.into()
    }

    #[test]
    fn test_linkednode() {
        with_externalities(&mut new_test_ext(), || {
            let mut node0 = Node::new(Order { id: 0, data: 0 });
            let node1 = Node::new(Order { id: 1, data: 1 });
            let node2 = Node::new(Order { id: 2, data: 2 });
            let node3 = Node::new(Order { id: 3, data: 3 });
            let node4 = Node::new(Order { id: 4, data: 4 });

            // add
            // 0
            node0.init_storage::<LinkedNodes<Test>>();
            assert_eq!(NodeHeader::<Test>::get().index(), 0);
            assert_eq!(NodeTail::<Test>::get().index(), 0);
            assert_eq!(NodeMap::<Test>::get(0).prev(), None);
            assert_eq!(NodeMap::<Test>::get(0).next(), None);

            // 1 0
            node0.add_node_before::<LinkedNodes<Test>>(node1).unwrap();
            assert_eq!(NodeHeader::<Test>::get().index(), 1);
            assert_eq!(NodeTail::<Test>::get().index(), 0);
            assert_eq!(NodeMap::<Test>::get(0).prev(), Some(1));
            assert_eq!(NodeMap::<Test>::get(0).next(), None);
            assert_eq!(NodeMap::<Test>::get(1).prev(), None);
            assert_eq!(NodeMap::<Test>::get(1).next(), Some(0));

            // 1 0 2
            node0.add_node_after::<LinkedNodes<Test>>(node2).unwrap();
            assert_eq!(NodeHeader::<Test>::get().index(), 1);
            assert_eq!(NodeTail::<Test>::get().index(), 2);
            assert_eq!(NodeMap::<Test>::get(0).prev(), Some(1));
            assert_eq!(NodeMap::<Test>::get(0).next(), Some(2));
            assert_eq!(NodeMap::<Test>::get(2).prev(), Some(0));
            assert_eq!(NodeMap::<Test>::get(2).next(), None);

            // 1 0 3 2
            let mut node2 = NodeMap::<Test>::get(2);
            node2.add_node_before::<LinkedNodes<Test>>(node3).unwrap();
            assert_eq!(NodeHeader::<Test>::get().index(), 1);
            assert_eq!(NodeTail::<Test>::get().index(), 2);
            assert_eq!(NodeMap::<Test>::get(0).prev(), Some(1));
            assert_eq!(NodeMap::<Test>::get(0).next(), Some(3));
            assert_eq!(NodeMap::<Test>::get(2).prev(), Some(3));
            assert_eq!(NodeMap::<Test>::get(2).next(), None);
            assert_eq!(NodeMap::<Test>::get(3).prev(), Some(0));
            assert_eq!(NodeMap::<Test>::get(3).next(), Some(2));

            // 1 4 0 3 2
            let mut node1 = NodeMap::<Test>::get(1);
            node1.add_node_after::<LinkedNodes<Test>>(node4).unwrap();
            assert_eq!(NodeHeader::<Test>::get().index(), 1);
            assert_eq!(NodeTail::<Test>::get().index(), 2);
            assert_eq!(NodeMap::<Test>::get(0).prev(), Some(4));
            assert_eq!(NodeMap::<Test>::get(0).next(), Some(3));
            assert_eq!(NodeMap::<Test>::get(1).prev(), None);
            assert_eq!(NodeMap::<Test>::get(1).next(), Some(4));
            assert_eq!(NodeMap::<Test>::get(4).prev(), Some(1));
            assert_eq!(NodeMap::<Test>::get(4).next(), Some(0));

            // remove_node
            // (1) 4 0 3 2
            let mut node1 = NodeMap::<Test>::get(1);
            node1.remove_node::<LinkedNodes<Test>>().unwrap();
            assert_eq!(NodeHeader::<Test>::get().index(), 4);
            assert_eq!(NodeMap::<Test>::get(4).prev(), None);

            // 4 0 3 (2)
            let mut node2 = NodeMap::<Test>::get(2);
            node2.remove_node::<LinkedNodes<Test>>().unwrap();
            assert_eq!(NodeTail::<Test>::get().index(), 3);
            assert_eq!(NodeMap::<Test>::get(3).next(), None);

            // 4 (0) 3
            let mut node0 = NodeMap::<Test>::get(0);
            node0.remove_node::<LinkedNodes<Test>>().unwrap();
            assert_eq!(NodeHeader::<Test>::get().index(), 4);
            assert_eq!(NodeMap::<Test>::get(4).next(), Some(3));
            assert_eq!(NodeTail::<Test>::get().index(), 3);
            assert_eq!(NodeMap::<Test>::get(3).prev(), Some(4));

            // (4) 3
            let mut node4 = NodeMap::<Test>::get(4);
            node4.remove_node::<LinkedNodes<Test>>().unwrap();
            assert_eq!(NodeHeader::<Test>::get().index(), 3);
            assert_eq!(NodeTail::<Test>::get().index(), 3);
            assert_eq!(NodeMap::<Test>::get(3).next(), None);

            // (3)
            let mut node3 = NodeMap::<Test>::get(3);
            node3.remove_node::<LinkedNodes<Test>>().unwrap();
            assert_eq!(NodeHeader::<Test>::exists(), false);
            assert_eq!(NodeTail::<Test>::exists(), false);
        })
    }

    #[test]
    fn test_linkedoptionnode() {
        with_externalities(&mut new_test_ext(), || {
            let mut node0 = Node::new(Order { id: 0, data: 0 });
            let node1 = Node::new(Order { id: 1, data: 1 });
            let node2 = Node::new(Order { id: 2, data: 2 });
            let node3 = Node::new(Order { id: 3, data: 3 });
            let node4 = Node::new(Order { id: 4, data: 4 });

            // add
            // 0
            node0.init_storage::<LinkedOptionNodes<Test>>();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 0);
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 0);
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().prev(), None);
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().next(), None);

            // 1 0
            node0.add_option_node_before::<LinkedOptionNodes<Test>>(node1).unwrap();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 1);
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 0);
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().prev(), Some(1));
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().next(), None);
            assert_eq!(OpNodeMap::<Test>::get(1).unwrap().prev(), None);
            assert_eq!(OpNodeMap::<Test>::get(1).unwrap().next(), Some(0));

            // 1 0 2
            node0.add_option_node_after::<LinkedOptionNodes<Test>>(node2).unwrap();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 1);
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 2);
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().prev(), Some(1));
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().next(), Some(2));
            assert_eq!(OpNodeMap::<Test>::get(2).unwrap().prev(), Some(0));
            assert_eq!(OpNodeMap::<Test>::get(2).unwrap().next(), None);

            // 1 0 3 2
            let mut node2 = OpNodeMap::<Test>::get(2).unwrap();
            node2.add_option_node_before::<LinkedOptionNodes<Test>>(node3).unwrap();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 1);
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 2);
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().prev(), Some(1));
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().next(), Some(3));
            assert_eq!(OpNodeMap::<Test>::get(2).unwrap().prev(), Some(3));
            assert_eq!(OpNodeMap::<Test>::get(2).unwrap().next(), None);
            assert_eq!(OpNodeMap::<Test>::get(3).unwrap().prev(), Some(0));
            assert_eq!(OpNodeMap::<Test>::get(3).unwrap().next(), Some(2));

            // 1 4 0 3 2
            let mut node1 = OpNodeMap::<Test>::get(1).unwrap();
            node1.add_option_node_after::<LinkedOptionNodes<Test>>(node4).unwrap();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 1);
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 2);
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().prev(), Some(4));
            assert_eq!(OpNodeMap::<Test>::get(0).unwrap().next(), Some(3));
            assert_eq!(OpNodeMap::<Test>::get(1).unwrap().prev(), None);
            assert_eq!(OpNodeMap::<Test>::get(1).unwrap().next(), Some(4));
            assert_eq!(OpNodeMap::<Test>::get(4).unwrap().prev(), Some(1));
            assert_eq!(OpNodeMap::<Test>::get(4).unwrap().next(), Some(0));

            // remove_node
            // (1) 4 0 3 2
            let mut node1 = OpNodeMap::<Test>::get(1).unwrap();
            node1.remove_option_node::<LinkedOptionNodes<Test>>().unwrap();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 4);
            assert_eq!(OpNodeMap::<Test>::get(4).unwrap().prev(), None);

            // 4 0 3 (2)
            let mut node2 = OpNodeMap::<Test>::get(2).unwrap();
            node2.remove_option_node::<LinkedOptionNodes<Test>>().unwrap();
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 3);
            assert_eq!(OpNodeMap::<Test>::get(3).unwrap().next(), None);

            // 4 (0) 3
            let mut node0 = OpNodeMap::<Test>::get(0).unwrap();
            node0.remove_option_node::<LinkedOptionNodes<Test>>().unwrap();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 4);
            assert_eq!(OpNodeMap::<Test>::get(4).unwrap().next(), Some(3));
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 3);
            assert_eq!(OpNodeMap::<Test>::get(3).unwrap().prev(), Some(4));

            // (4) 3
            let mut node4 = OpNodeMap::<Test>::get(4).unwrap();
            node4.remove_option_node::<LinkedOptionNodes<Test>>().unwrap();
            assert_eq!(OpNodeHeader::<Test>::get().unwrap().index(), 3);
            assert_eq!(OpNodeTail::<Test>::get().unwrap().index(), 3);
            assert_eq!(OpNodeMap::<Test>::get(3).unwrap().next(), None);

            // (3)
            let mut node3 = OpNodeMap::<Test>::get(3).unwrap();
            node3.remove_option_node::<LinkedOptionNodes<Test>>().unwrap();
            assert_eq!(OpNodeHeader::<Test>::exists(), false);
            assert_eq!(OpNodeTail::<Test>::exists(), false);
        })
    }

    #[test]
    fn test_linked_multi_index_node() {
        with_externalities(&mut new_test_ext(), || {
            let mut node0 = Node::new(Order { id: 0, data: 0 });
            let node1 = Node::new(Order { id: 1, data: 1 });
            let node2 = Node::new(Order { id: 2, data: 2 });
            let node3 = Node::new(Order { id: 3, data: 3 });
            let mut node4 = Node::new(Order { id: 4, data: 4 });

            // add
            // 0
            node0.init_storage_withkey::<LinkedOptionMultiKey<Test>, u32>(10);
            // 4
            node4.init_storage_withkey::<LinkedOptionMultiKey<Test>, u32>(99);
            // 2 0 1
            node0.add_option_node_after_withkey::<LinkedOptionMultiKey<Test>, u32>(node1, 10).unwrap();
            node0.add_option_node_before_withkey::<LinkedOptionMultiKey<Test>, u32>(node2, 10).unwrap();

            node4.add_option_node_before_withkey::<LinkedOptionMultiKey<Test>, u32>(node3, 99).unwrap();

            // test key 10
            let test_v = [2_u64, 0, 1];
            let mut index = Module::<Test>::multi_header(10).unwrap().index();
            let mut v = vec![];
            loop {
                if let Some(node) = Module::<Test>::op_node_map2(&index) {
                    v.push(node.index());
                    if let Some(next) = node.next() {
                        index = next;
                    } else { break; }
                } else { break; }
            }
            assert_eq!(v.as_slice(), test_v);

            let test_v = [3_u64, 4];
            let mut index = Module::<Test>::multi_header(99).unwrap().index();
            let mut v = vec![];
            loop {
                if let Some(node) = Module::<Test>::op_node_map2(&index) {
                    v.push(node.index());
                    if let Some(next) = node.next() {
                        index = next;
                    } else { break; }
                } else { break; }
            }
            assert_eq!(v.as_slice(), test_v);

            let r = Module::<Test>::multi_tail(50) == None;
            assert_eq!(r, true);

            // key 99 tail
            let index = Module::<Test>::multi_tail(99).unwrap().index();
            assert_eq!(index, 4);
            // 3 (4)
            let mut node4 = Module::<Test>::op_node_map2(index).unwrap();
            assert_eq!(node4.index(), 4);
            node4.remove_option_node_withkey::<LinkedOptionMultiKey<Test>, u32>(99).unwrap();
            let node3 = Module::<Test>::op_node_map2(3).unwrap();
            assert_eq!(node3.prev(), None);
            assert_eq!(node3.next(), None);
            assert_eq!(Module::<Test>::multi_header(99).unwrap().index(), node3.index());
            assert_eq!(Module::<Test>::multi_tail(99).unwrap().index(), node3.index());

            let index = Module::<Test>::multi_header(99).unwrap().index();
            assert_eq!(node3.index(), 3);
            let mut node3 = Module::<Test>::op_node_map2(index).unwrap();
            assert_eq!(node3.index(), 3);
            node3.remove_option_node_withkey::<LinkedOptionMultiKey<Test>, u32>(99).unwrap();
            assert_eq!(node3.prev(), None);
            assert_eq!(node3.next(), None);
            assert_eq!(Module::<Test>::multi_header(99) == None, true);
            assert_eq!(Module::<Test>::multi_tail(99) == None, true);
        })
    }
}
