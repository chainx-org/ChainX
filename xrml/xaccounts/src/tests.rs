// Copyright 2018-2019 Chainpool.

#![cfg(test)]

use runtime_io::with_externalities;
use support::StorageMap;
use support::{assert_noop, assert_ok};
use xsystem::Validator;

use super::mock::{new_test_ext, Test, XAccounts};
use super::*;

#[test]
fn test_is_valid_name() {
    assert_ok!(is_valid_name("na".as_bytes()));
    assert_ok!(is_valid_name("nam".as_bytes()));
    assert_ok!(is_valid_name("name".as_bytes()));
    assert_ok!(is_valid_name("namenamename".as_bytes()));
    assert_noop!(
        is_valid_name("".as_bytes()),
        "The length of name must be in range [2, 12]."
    );
    assert_noop!(
        is_valid_name("n".as_bytes()),
        "The length of name must be in range [2, 12]."
    );
    assert_noop!(
        is_valid_name("namenamename_".as_bytes()),
        "The length of name must be in range [2, 12]."
    );
    assert_noop!(
        is_valid_name("<script>".as_bytes()),
        "'<' and '>' are not allowed, which could be abused off-chain."
    );
}

#[test]
fn test_is_valid_about() {
    assert_ok!(is_valid_about("".as_bytes()));
    assert_ok!(is_valid_about("about".as_bytes()));
    assert_ok!(is_valid_about("abcd".repeat(32).as_bytes()));
    assert_noop!(
        is_valid_about("abcd".repeat(33).as_bytes()),
        "The length of about must be in range [0, 128]."
    );
    assert_noop!(
        is_valid_about("<script>".as_bytes()),
        "'<' and '>' are not allowed, which could be abused off-chain."
    );
}

#[test]
fn test_is_valid_url() {
    assert_ok!(is_valid_url(".".repeat(4).as_bytes()));
    assert_ok!(is_valid_url("1".repeat(4).as_bytes()));
    assert_ok!(is_valid_url("a.b.c".as_bytes()));
    assert_ok!(is_valid_url("aaa.bbb.ccc".as_bytes()));
    assert_ok!(is_valid_url("aaa.aaa.bbb.bbb.ccc.cccc".as_bytes()));
    assert_ok!(is_valid_url("..12..34..abc..".as_bytes()));

    assert_noop!(
        is_valid_url(".".repeat(3).as_bytes()),
        "The length of url must be in range [4, 24]."
    );
    assert_noop!(
        is_valid_url("1".repeat(3).as_bytes()),
        "The length of url must be in range [4, 24]."
    );
    assert_noop!(
        is_valid_url("a".repeat(3).as_bytes()),
        "The length of url must be in range [4, 24]."
    );
    assert_noop!(
        is_valid_url("abcde".repeat(5).as_bytes()),
        "The length of url must be in range [4, 24]."
    );
    assert_noop!(
        is_valid_url("http://a.b.c".as_bytes()),
        "Only ASCII alphanumeric character and . are allowed."
    );
    assert_noop!(
        is_valid_url("https://a.b.c".as_bytes()),
        "Only ASCII alphanumeric character and . are allowed."
    );
}

#[test]
fn validator_should_work() {
    with_externalities(&mut new_test_ext(), || {
        assert_eq!(XAccounts::intention_of(b"test".to_vec()), None);
        assert_eq!(XAccounts::intention_name_of(1), None);
        <IntentionOf<Test>>::insert(b"test".to_vec(), 1);
        <IntentionNameOf<Test>>::insert(1, b"test".to_vec());
        assert_eq!(XAccounts::get_validator_by_name(&b"test".to_vec()), Some(1));
        assert_eq!(XAccounts::get_validator_name(&1), Some(b"test".to_vec()));
    });
}
