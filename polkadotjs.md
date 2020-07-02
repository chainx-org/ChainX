# json type for inject into polkadot.js
## type support
```json
{
  "Address": "AccountId",
  "Token": "Text",
  "Desc": "Text",
  "Memo": "Text",
  "AddrStr": "Text",
  "NetworkType": {
    "_enum": [
      "Mainnet",
      "Testnet"
    ]
  },
  "Chain": {
      "_enum": [
        "ChainX",
        "Bitcoin",
        "Ethereum",
        "Polkadot"
      ]
  },
  "Precision": "u8",
  "AssetId": "u32",
  "AssetInfo": {
    "token": "Token",
    "token_name": "Token",
    "chain": "Chain",
    "precision": "Precision",
    "desc": "Desc"       
  },
  "AssetType": {
    "_enum": [
      "Free",
      "ReservedStaking",
      "ReservedStakingRevocation",
      "ReservedWithdrawal",
      "ReservedDexSpot",
      "ReservedDexFuture",
      "ReservedCurrency",
      "ReservedXRC20"
    ]
  },
  "AssetRestrictions": "u32",
  "AssetRestriction": {
    "_enum": [
      "Move",
      "Transfer",
      "Deposit",
      "Withdraw",
      "DestroyWithdrawal",
      "DestroyFree"
    ]
  },
  "SignedBalance": {
    "_enum": {
      "Positive": "Balance",
      "Negative": "Balance"
    }
  },
  "Compact": "u32",
  "BTCHeader": {
    "version": "u32",
    "previous_header_hash": "H256",
    "merkle_root_hash": "H256",
    "time": "u32",
    "bits": "Compact",
    "once": "u32"
  },
  "BTCHeaderInfo": {
    "header": "BTCHeader",
    "height": "u32",
    "confirmed": "bool",
    "txid_list": "Vec<H256>"
  },
  "OutPoint": {
    "hash": "H256",
    "index": "u32"
  },
  "TransactionInput": {
    "previous_output": "OutPoint",
    "script_sig": "Bytes",
    "sequence": "u32",
    "script_witness": "Vec<Bytes>"
  },
  "TransactionOutput": {
    "value": "u64",
    "script_pubkey": "Bytes"
  },
  "BTCTransaction": {
    "version": "i32",
    "inputs": "Vec<TransactionInput>",
    "outputs": "Vec<TransactionOutput>",
    "lock_time": "u32"
  },
  "BTCTxType": {
    "_enum": [
      "Withdrawal",
      "Deposit",
      "HotAndCold",
      "TrusteeTransition",
      "Lock",
      "Unlock",
      "Irrelevance"
    ]
  },
  "BTCTxInfo": {
    "raw_tx": "BTCTransaction",
    "tx_type": "BTCTxType",
    "height": "u32"
  },
  "BTCAddrTyep": {
    "_enum": [
      "P2PKH",
      "P2SH"
    ]
  },
  "BTCNetwork": {
     "_enum": [
       "Mainnet",
       "Testnet"
     ]
  },
  "AddressHash": "H160",
  "BTCAddress": {
    "kind": "Type",
    "network": "Network",
    "hash": "AddressHash"
  },
  "BTCParams": {
    "max_bits": "u32",
    "block_max_future": "u32",
    "target_timespan_seconds": "u32",
    "target_spacing_seconds": "u32",
    "retargeting_factor": "u32",
    "retargeting_interval": "u32",
    "min_timespan": "u32",
    "max_timespan": "u32"
  },
  "ContractInfo": "RawAliveContractInfo",
  "XRC20Selector": {
    "_enum": [
      "BalanceOf",
      "TotalSupply",
      "Name",
      "Symbol",
      "Decimals",
      "Issue",
      "Destroy"
    ]
  },
  "Selector": "[u8; 4]",
  "AssetInfoForRpc": {
      "token": "String",
      "token_name": "String",
      "chain": "Chain",
      "precision": "Precision",
      "desc": "String"
  },
  "TotalAssetInfoForRpc": {
    "info": "AssetInfoForRpc",
    "balance": "BTreeMap<AssetType, String>",
    "isOnline": "bool",
    "restrictions": "AssetRestrictions"
  }
}
```

## Rpc
```ts
rpc: {
  xassets: {
    getAssetsByAccount: {
      description: 'get all assets balance for an account',
      params: [
        {
            name: 'who',
            type: 'AccountId'
        },
        {
          name: 'at',
          type: 'Hash',
          isOptional: true
        }
      ],
      type: 'BTreeMap<AssetId, BTreeMap<AssetType, String>>'
    },
    getAssets: {
      description: 'get all assets balance and infos',
      params: [
        {
          name: 'at',
          type: 'Hash',
          isOptional: true
        }
      ],
      type: 'BTreeMap<AssetId, TotalAssetInfoForRpc>'
    }
  }
}
```