# ChainX

## Single validator:
```
   RUST_LOG=info ./target/debug/chainx --chainspec=dev validator --auth=alice
```
## Local test(2 validator)
- first node
```
   RUST_LOG=info ./target/debug/chainx --chainspec=local validator --auth=alice
```
- second node
```
   RUST_LOG=info ./chainx --chainspec=local --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/QmbrhgywtX5eL66rboxBKg7p4kW5RSiEg1djDuqrxAfEFW validator --auth=bob
```

## Multi test(4 validator)
- 1
```
   RUST_LOG=info ./target/debug/chainx --chainspec=multi validator --auth=alice
```
- 2
```
   RUST_LOG=info ./chainx --chainspec=multi --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/QmWrZEJcYn3m8HeiHsYDVH1apitFF1h4ojyRYu9AjFkTuH validator --auth=bob
```
- 3
```
RUST_LOG=info ./chainx --chainspec=multi --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/QmWrZEJcYn3m8HeiHsYDVH1apitFF1h4ojyRYu9AjFkTuH validator --auth=gavin
```
- 4
```
./chainx --chainspec=multi --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/QmWrZEJcYn3m8HeiHsYDVH1apitFF1h4ojyRYu9AjFkTuH validator --auth=satoshi
```
