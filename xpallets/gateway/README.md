# Overview

This is the btc cross-chain development document. Introduce related storage and rpc interface through case.

# Case

## 成为信托

### 成为议会成员

1. 要成为议会成员，首先要成为候选人

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399971419031639997141896.png)

2. 任何人都可以给候选人进行投票。投票可以给多个人投票需要质押一定的ksx。议会的换届周期为一天。每一天会根据得票数和相关的质押ksx计算排名。选前11名作为议会成员，后7人作为runners up.

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399970352971639997035289.png)

### 设置信托成员信息

议会当前成员以及runner up中的成员都是信托候选人员。信托候选人员需要提前设置信托信息，否则无法参选信托。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399849656191639984965606.png)

- proxy_account : 不填默认为和议会账户是相同的。主要作用是减少议会账户的使用，信托换届相关的交易都可通过代理账户进行操作

- chain: 目前填`Bitcoin`

- about: 备注 

- hot_entity: 填写BTC的公钥 （用来签名提现交易的账号）。压缩公钥和无压缩公钥均可。如`0x02926877f1a4c5e348c32ab6307799f8ac6836bf60a2c3a38e56a759cabe8f0187`.

- cold_entity: 填写BTC的公钥 （使用少、安全性更高，用于存储BTC）。压缩公钥和无压缩公钥均可。

  如`039392e66cb126ce7116a4dacd2682ddd80721f951b106818b03fea3e836713d12`.

## 充值

充值就是由用户向当前届信托的热地址进行转账，需要带上OP_RETURN(即用户的Sherpax账户信息)。

### 查询当前届信托热地址

#### xgatewaycommon_bitcoinTrusteeSessionInfo

参数：负数：-1表示查询当前届信托信息，-2表示查询上一届信托信息。正数：表示具体某届的信托信息。

返回值：

- coldAddress: 信托冷地址
- hotAddress: 信托热地址
- multiAccount: 信托sherpax多签账户
- threshold: 阈值
- trusteeList: 信托相关的所有成员账户和他们参与提现的累积权重（参与提现的btc累加）
- startHeight: 成为信托的起始块
- en dHeight: 信托换届的结束块

![img](https://cdn.nlark.com/yuque/0/2021/png/1606853/1639997268585-039af3ee-e50c-4fa3-9d3c-dd089af3d79b.png)

### 查询最小充值金额

用户充值金额小于最小充值金额等于没冲

### ![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399902138761639990213872.png)

### 测试的交易

```
// 交易大致信息
From：tb1p8pvzqnc46wza5ahuhhcpnh4aucjx383fd3dv20myxay322y9wctsj9qznh
To：tb1p4se6umxrsfqyaumx2qzzl75ahfrjshf52xpkas0gk4ytgx2qtyqs5zyuse
OP_RETURN:354468616370794132596b706a783441554a4762463771613874507146454c4556515958517378585153617550623972

Input: 
● txid: 5c46f3a45947443cb0592919893a167584e01902fe3b19a3e60e91bccf33ecff
● vout: 0
Output:
● [0.0998, tb1p4se6umxrsfqyaumx2qzzl75ahfrjshf52xpkas0gk4ytgx2qtyqs5zyuse]
● [0, OP_RETURN:354468616370794132596b706a783441554a4762463771613874507146454c4556515958517378585153617550623972]
// 构造出的交易
txid:3af6ff1d16e38c349cc87da12b4b6fae518840a94c63207e8ada946b49c6b3c1
tx:02000000000101ffec33cfbc910ee6a3193bfe0219e08475163a89192959b03c444759a4f3465c020000000000000000026048980000000000225120ac33ae6cc382404ef36650042ffa9dba47285d3451836ec1e8b548b4194059010000000000000000326a30354468616370794132596b706a783441554a4762463771613874507146454c45565159585173785851536175506239720140f98488c405374730ed0179d59de361d47bcb0317503500885ab1c767ebdabd5eaf56da3f21fc2313a928130f2659729f146eef876656fffc42676e39b736d39d00000000

scan:https://signet.bitcoinexplorer.org/tx/3af6ff1d16e38c349cc87da12b4b6fae518840a94c63207e8ada946b49c6b3c1
```

## 提现

### Step 1

#### 用户申请提现![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399808590601639980859053.png)

- assetId：默认sbtc是1

- value：提现的金额（BTC的精度为8位，5000000相当于0.05BTC）

- addr: 要提现到的账号（BTC地址）

- ext：备注（无影响）

#### 查询用户锁定资产

用户申请提现后，提现的金额会进入锁定状态，查询用户锁定资产数量。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399903900851639990390083.png)

- AccountId：填用户账户

- u32: 为资产ID，sbtc默认为1

#### 查询最小提现金额

确保提现金额要大于链上最小提现金额

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399901113041639990111299.png)

### Step 2

#### 信托查询所有提现列表

用户申请提现后，信托要根据用户的提现信息来构造提现交易，因此首先要查询所有未处理的用户提现

##### xgatewayrecords_withdrawalList

参数：无

返回值： 字典(`withdraw_id` -> `withdraw_info`)

- addr: 用户提现接收的btc地址
- applicant: 用户sherpax账户
- assetId: 资产id, 1表示btc
- height: 用户申请提现的块高度 
- state: 用户申请先的状态：`Applying`表示未处理的提现申请。`Processing`表示正在处理的提现申请。`NormalFinish/RootFinish`表示已经完成的提现申请。`NormalCancel/RootCancel`表示已经取消的提现申请。
- balance：提现数量

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399936095111639993609503.png)

#### 信托链下构造交易

利用上述查询到的所有`Applying`申请构造提现交易，从信托的热地址进行门限签名转账. 接收方地址为上述查询到的addr，数量为上述`balance-提现手续费`

##### 查询提现手续费

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400554712471640055471241.png)

唯一的参数是资产ID，默认使用1就是sbtc

#### 信托提交提现

构造完交易后，不能直接向btc网络广播交易，而是向sherpax广播交易，带上相关的withdraw id。否则sherpax无法确认是哪几笔交易正在进行提现并设定提现的状态。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399809713971639980971389.png)

- withdrawList: 即`信托链下构造交易`所采用的`Applying`状态的`withdraw_id`

### 测试的交易

```
// 交易大致信息
From: tb1p4se6umxrsfqyaumx2qzzl75ahfrjshf52xpkas0gk4ytgx2qtyqs5zyuse
To: tb1p8pvzqnc46wza5ahuhhcpnh4aucjx383fd3dv20myxay322y9wctsj9qznh
Input: 
txid: 3af6ff1d16e38c349cc87da12b4b6fae518840a94c63207e8ada946b49c6b3c1
    witness: 
        scritpubkey: 
        signature:
        control:

Output:
	- [0.05, tb1p8pvzqnc46wza5ahuhhcpnh4aucjx383fd3dv20myxay322y9wctsj9qznh]
	- [0.0497, tb1p4se6umxrsfqyaumx2qzzl75ahfrjshf52xpkas0gk4ytgx2qtyqs5zyuse]

// 构造出的交易
txid:c0ac3506319f0b2659887aca9beea16bd1ea967c40241a3678d19857ce5ecc30

tx:02000000000101c1b3c6496b94da8a7e20634ca9408851ae6f4b2ba17dc89c348ce3161dfff63a00000000000000000002404b4c00000000002251203858204f15d385da76fcbdf019debde624689e296c5ac53f643749152885761710d64b0000000000225120ac33ae6cc382404ef36650042ffa9dba47285d3451836ec1e8b548b41940590103406a00831ae0c6f552d6b607e567f27ac99ceaf992e934ac15341b344a7ad16414b7a836d595e295ad5627a8e849ff357a26726fd87e5a9c717026010e34f1ac3b2220b7d324c0d6a6040bed5943df22dee756270461ce0dc8dd8244df6f4c7916d9b6ac61c157f953664f15498ff244bea9b4fbc844bd765622557579fb36ff1a0dcf30bc1e1800a226d95208191a226ddcc71cff531a2564e48403ff0e2bd2cdd85841258b1a61293b0491eb083beece213296174b85318b9c8b63dc1e985a4de2e6004d9500000000

spent_output:016048980000000000225120ac33ae6cc382404ef36650042ffa9dba47285d3451836ec1e8b548b419405901

scan:https://signet.bitcoinexplorer.org/tx/c0ac3506319f0b2659887aca9beea16bd1ea967c40241a3678d19857ce5ecc30
```

## 信托换届

信托换届的过程主要分成两大步。第一步是每个月到期后sherpax**自动**进行拉取议会相关信息并进行信托换届，并将信托换届状态设置为True。第二步是上一届信托需构造从上一届信托冷热地址到当前届信托热冷地址的比特币转账交易。

### 查询信托换届状态

如果为True,表示信托需要进行信托换届，即信托换届第一步已完成，需要进行第二步，从（上一届冷地址-->当前届冷地址， 上一届热地址-->当前届热地址）的转账。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399909392371639990939232.png)

参考`xgatewaycommon_bitcoinTrusteeSessionInfo`,传入参数-1查询当前届的信托冷热地址，上一届信托成员通过门限签名构造一笔转账交易。

### 测试的交易

1. 换届的过程是发起一笔从旧的旧热地址像新的旧热地址转账的交易，

```
// 当前届信托热、冷地址 (Alice、Bob、Charlie)
hot_address:tb1p4se6umxrsfqyaumx2qzzl75ahfrjshf52xpkas0gk4ytgx2qtyqs5zyuse
cold_address:tb1pmzfty9r4m4h5gnl0env7lxcu5kud6gu57syt94mezp4arfc5fk3qr3stqh
// 换届后信托热、冷地址 (Alice、Bob、Dave)
hot_address:tb1pf23c3alq9sp2ylg4tv233m0tng2v98xldvzy6q52tnwah39n9fgqqrkuum
cold_address:tb1p9jkustqjpqeeayl8p74u6rnpsf3pqp4ac3n2uygsxdtkyrkvjzwspwn8gj
```

2. 换届交易原文

```
旧热地址->新热地址
旧热地址：tb1p4se6umxrsfqyaumx2qzzl75ahfrjshf52xpkas0gk4ytgx2qtyqs5zyuse
新热地址：tb1pf23c3alq9sp2ylg4tv233m0tng2v98xldvzy6q52tnwah39n9fgqqrkuum
// 交易大致信息
From: tb1p4se6umxrsfqyaumx2qzzl75ahfrjshf52xpkas0gk4ytgx2qtyqs5zyuse
To: tb1pf23c3alq9sp2ylg4tv233m0tng2v98xldvzy6q52tnwah39n9fgqqrkuum
Input: 
txid: c0ac3506319f0b2659887aca9beea16bd1ea967c40241a3678d19857ce5ecc30
witness: scritpubkey: 
signature:
control:

Output:
target: tb1pf23c3alq9sp2ylg4tv233m0tng2v98xldvzy6q52tnwah39n9fgqqrkuum
amount: 0.0496

// 构造出的交易
txid:840a8f49d8dbd88b623cbbd13b72a487e2f1ca06550b5719ca51e5334fd2905a
tx:0200000000010130cc5ece5798d178361a24407c96ead16ba1ee9bca7a8859260b9f310635acc00100000000000000000100af4b00000000002251204aa388f7e02c02a27d155b1518edeb9a14c29cdf6b044d028a5cdddbc4b32a50034065bce089f238132e9e2ff5a4f199a64a6ca356448ae91b78c2d030a05283cbfcf4f692ad395c144eb4c4d22bebe7581df68d64d9752895eaa6ec38b576e900de2220b7d324c0d6a6040bed5943df22dee756270461ce0dc8dd8244df6f4c7916d9b6ac61c157f953664f15498ff244bea9b4fbc844bd765622557579fb36ff1a0dcf30bc1e1800a226d95208191a226ddcc71cff531a2564e48403ff0e2bd2cdd85841258b1a61293b0491eb083beece213296174b85318b9c8b63dc1e985a4de2e6004d9500000000

scan:https://signet.bitcoinexplorer.org/tx/840a8f49d8dbd88b623cbbd13b72a487e2f1ca06550b5719ca51e5334fd2905a
```

# 信托奖励分发

### 申请奖励

1.每个月上一届信托可以向国库申请`10500000/12*0.05=43750` ksx奖励到多签账户。多签账户的查询参考`xgatewaycommon_bitcoinTrusteeSessionInfo`，传入参数-2查询上一届信托信息(如下图所示)。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400561829991640056182992.png)

### 奖励分配

奖励申请到账后，上一届信托成员中任一人通过下图所示交易进行奖励分配，最终奖励会分发到各个信托的账户中。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400556024121640055602407.png)

- sessionNum: 负数：-1表示查询当前届信托信息，-2表示查询上一届信托信息。正数：表示具体某届的信托信息
