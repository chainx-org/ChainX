// Copyright 2018 Chainpool.

use super::*;
use rstd::prelude::Vec;

/// OP_RETURN extracter
pub struct Extracter<'a>(&'a [u8]);

impl<'a> Extracter<'a> {
    pub fn new(script: &[u8]) -> Extracter {
        Extracter(script)
    }

    fn split(&self) -> Vec<Vec<u8>> {
        let s = self.0;
        let mut iter = s.split(|x| *x == ':' as u8);
        let mut v = Vec::new();
        while let Some(d) = iter.next() {
            let d: Vec<u8> = d.iter().cloned().collect();
            v.push(d)
        }
        v
    }

    fn quick_check(v: &Vec<Vec<u8>>) -> bool {
        if v.len() < 1 {
            return false;
        }

        let chainx = &v[0];
        let chainx = &chainx[2..];

        if chainx != OP_RETURN_FLAG {
            return false;
        }

        true
    }

    pub fn account_id<T: Trait>(self) -> Option<T::AccountId> {
        let v = self.split();
        if !Self::quick_check(&v) {
            return None;
        }

        let account = &v[1];
        let account_id: Option<T::AccountId> = Decode::decode(&mut account.as_slice());

        account_id
    }

    pub fn cert<T: Trait>(self) -> Option<(Vec<u8>, u32, T::AccountId)> {
        let v = self.split();

        if !Self::quick_check(&v) {
            return None;
        }

        let account = &v[1];
        let account_id: T::AccountId = match Decode::decode(&mut account.as_slice()) {
            Some(a) => a,
            None => return None,
        };

        let cert_name = &v[2];
        let duration = &v[3];

        // TODO to eliminate vec_to_u32
        // let days = u32::from_be_bytes(&v[3]);

        let frozen_duration = vec_to_u32(duration.to_vec()).unwrap_or(0);
        if frozen_duration <= 0 {
            return None;
        }

        Some((cert_name.to_vec(), frozen_duration, account_id))
    }
}

pub fn vec_to_u32(date: Vec<u8>) -> Option<u32> {
    let mut frozen_duration = 0;
    let maxvalue = u32::max_value();
    // ascii '0' = 48  '9' = 57
    for i in date {
        if i > 57 || i < 48 {
            return None;
        }
        frozen_duration = frozen_duration * 10 + u32::from(i - 48);
        if frozen_duration > maxvalue {
            return None;
        }
    }
    Some(frozen_duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use b58::from;

    #[test]
    fn test_vec_to_u32() {
        let mut date: Vec<u8> = Vec::new();
        date.push(54);
        date.push(54);
        date.push(57);
        date.push(57);
        date.push(48);

        let frozen_duration = if let Some(date) = vec_to_u32(date) {
            date
        } else {
            0
        };
        assert_eq!(66990, frozen_duration);
    }

    #[test]
    fn test_account_id() {
        let script = Script::from(
            "01ChainX:5HnDcuKFCvsR42s8Tz2j2zLHLZAaiHG4VNyJDa7iLRunRuhM"
                .as_bytes()
                .to_vec(),
        );

        let s = script.to_bytes();
        let mut iter = s.as_slice().split(|x| *x == ':' as u8);
        let mut v = Vec::new();
        while let Some(d) = iter.next() {
            v.push(d);
        }
        assert_eq!(v.len(), 2);

        let chainx = v[0];
        assert_eq!(&chainx[2..], OP_RETURN_FLAG);

        let mut account = Vec::new();
        account.extend_from_slice(&v[1]);
        let slice: Vec<u8> = from(account).unwrap();
        let slice = slice.as_slice();
        let mut account: Vec<u8> = Vec::new();
        account.extend_from_slice(&slice[1..33]);
        assert_eq!(account.len(), 32);
        let account_id: H256 = Decode::decode(&mut account.as_slice()).unwrap();
        assert_eq!(
            account_id,
            H256::from("fcd66b3b5a737f8284fef82d377d9c2391628bbe11ec63eb372b032ce2618725")
        );
    }

    #[test]
    fn test_cert() {
        let script = Script::from(
            "01ChainX:5HnDcuKFCvsR42s8Tz2j2zLHLZAaiHG4VNyJDa7iLRunRuhM:certname:6"
                .as_bytes()
                .to_vec(),
        );

        let s = script.to_bytes();
        let mut iter = s.as_slice().split(|x| *x == ':' as u8);
        let mut v = Vec::new();
        while let Some(d) = iter.next() {
            v.push(d);
        }
        assert_eq!(v.len(), 4);

        let chainx = v[0];
        assert_eq!(&chainx[2..], OP_RETURN_FLAG);

        let mut account = Vec::new();
        account.extend_from_slice(&v[1]);
        let slice: Vec<u8> = from(account).unwrap();
        let slice = slice.as_slice();
        let mut account: Vec<u8> = Vec::new();
        account.extend_from_slice(&slice[1..33]);
        assert_eq!(account.len(), 32);
        let account_id: H256 = Decode::decode(&mut account.as_slice()).unwrap();
        assert_eq!(
            account_id,
            H256::from("fcd66b3b5a737f8284fef82d377d9c2391628bbe11ec63eb372b032ce2618725")
        );

        let duration = v[3];
        let frozen_duration = if let Some(duration) = vec_to_u32(duration.to_vec()) {
            duration
        } else {
            0
        };
        assert_eq!(6, frozen_duration);
    }
}
