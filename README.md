# ChainX

## Single node:
```
    RUST_LOG=info ./target/debug/chainx --dev validator -a
```
## Local test(2 nodes)
- first node
```
   RUST_LOG=info ./target/debug/chainx validator -a
```
- second node
```
  RUST_LOG=info ./chainx --bootnodes=/ip4/127.0.0.1/tcp/20222/p2p/Qmdn5LBru7mAPWDCBwEyUo6LVx4UuM2KqX4tdCfefef6zq validator
```
/ip4/127.0.0.1/tcp/20222/p2p/Qmdn5LBru7mAPWDCBwEyUo6LVx4UuM2KqX4tdCfefef6zq is first node url.
