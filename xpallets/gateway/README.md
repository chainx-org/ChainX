# Case

## 成为信托

### 成为议会成员

1. 要成为议会成员，首先要成为候选人

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399971419031639997141896.png)

2. 任何人都可以给候选人进行投票。投票可以给多个人投票需要质押一定的pcx。议会的换届周期为一天。每一天会根据得票数和相关的质押pcx计算排名。选前11名作为议会成员，后7人作为runners up.

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

充值就是由用户向当前届信托的热地址进行转账，需要带上OP_RETURN(即用户的chainx账户信息)。

### 查询当前届信托热地址

#### xgatewaycommon_bitcoinTrusteeSessionInfo

参数：负数：-1表示查询当前届信托信息，-2表示查询上一届信托信息。正数：表示具体某届的信托信息。

返回值：

- coldAddress: 信托冷地址
- hotAddress: 信托热地址
- multiAccount: 信托chainx多签账户
- threshold: 阈值
- trusteeList: 信托相关的所有成员账户和他们参与提现的累积权重（参与提现的btc累加）
- startHeight: 成为信托的起始块
- en dHeight: 信托换届的结束块

![img](https://cdn.nlark.com/yuque/0/2021/png/1606853/1639997268585-039af3ee-e50c-4fa3-9d3c-dd089af3d79b.png)

### 查询最小充值金额

用户充值金额小于最小充值金额等于没冲

### ![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399902138761639990213872.png)

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
- applicant: 用户chainx账户
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

构造完交易后，不能直接向btc网络广播交易，而是向chainx广播交易，带上相关的withdraw id。否则chainx无法确认是哪几笔交易正在进行提现并设定提现的状态。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399809713971639980971389.png)

- withdrawList: 即`信托链下构造交易`所采用的`Applying`状态的`withdraw_id`

## 信托换届

信托换届的过程主要分成两大步。第一步是每个月到期后chainx**自动**进行拉取议会相关信息并进行信托换届，并将信托换届状态设置为True。第二步是上一届信托需构造从上一届信托冷热地址到当前届信托热冷地址的比特币转账交易。

### 查询信托换届状态

如果为True,表示信托需要进行信托换届，即信托换届第一步已完成，需要进行第二步，从（上一届冷地址-->当前届冷地址， 上一届热地址-->当前届热地址）的转账。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16399909392371639990939232.png)

参考`xgatewaycommon_bitcoinTrusteeSessionInfo`,传入参数-1查询当前届的信托冷热地址，上一届信托成员通过门限签名构造一笔转账交易。

# 信托奖励分发

### 申请奖励

1.每个月上一届信托可以向国库申请pcx奖励到多签账户。多签账户的查询参考`xgatewaycommon_bitcoinTrusteeSessionInfo`，传入参数-2查询上一届信托信息(如下图所示)。如果一个月到期没有进行换届，也可以进行申请并分配奖励。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400561829991640056182992.png)

### 奖励分配

奖励申请到账后，上一届信托成员中任一人通过下图所示交易进行奖励分配，最终奖励会分发到各个信托的账户中。一个月没有换届，申请的奖励需要议会投票执行下图交易。

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400556024121640055602407.png)

- sessionNum: 负数：-1表示查询当前届信托信息，-2表示查询上一届信托信息。正数：表示具体某届的信托信息
