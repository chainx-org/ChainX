# BTC Ledger

BTC Ledger records all Bitcoin balances which transferred from Bitcoin to ChainX.

The users can see their bitcoin balance on **metamask**.

**bitcoin as evm gas**, they can interact with dapps on chainx-evm through **metamask**. 



## RPC
```json
"btcledger": {
        "getBalance": {
            "description": "get the btc balance of the account",
            "params": [
                {
                    "name": "who",
                    "type": "AccountId"
                },
                {
                    "name": "at",
                    "type": "Hash",
                    "isOptional": true
                }
            ],
            "type": "RpcBalance<Balance>"
        },
        "getTotalInComing": {
            "description": "get the total incoming BTC balance",
            "params": [
                {
                    "name": "at",
                    "type": "Hash",
                    "isOptional": true
                }
            ],
            "type": "RpcBalance<Balance>"
        }
    }
```