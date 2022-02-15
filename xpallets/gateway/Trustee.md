# Overview

This document describes how to become a member of council and trustee. Becoming a trust, the main responsibility is to keep the btc for users and help with withdrawals. Every month the trust can apply to the treasury for `10500000/12*0.05=43750` ksx as rewards. Rewards are distributed in proportion to the number of withdrawals and the number of BTCs that help users withdraw.

# Council

## Elected candidate

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400546556771640054655670.png)

To become a member of council, one must first be elected as a candidate.

## Vote

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400548057191640054805712.png)

Everyone can stake some **ksx** and vote for multiple candidates. Allow yourself to vote for yourself. **After becoming a candidate, members of the parliament will be updated every day, and the ranking will be calculated based on the number of votes and related staking ksx.**

# Trustee

To become a trust, you must first be elected as a council member or runners up. **Then set your own btc hot and cold public key as shown in the figure below.** The **hot public key** is used for general deposit and withdrawal, and the **cold public key** is used to store large amounts of btc to improve security. After becoming a trust and setting up the btc information, the trust will be renewed every 30 days.

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400549209591640054920952.png)

- proxy_account : An proxy account, if it is not filled in, the default is the same as the council account. Avoid frequent use of council accounts.
- chain: Fill `Bitcoin`
- about: Remark
- hot_entity: Btc public key. Such as `0x043858204f15d385da76fcbdf019debde624689e296c5ac53f6437491528857617691fe85c5c529b692bd75e361a9d0995dbd3e20a81e949642dfb74095520d981`.
- cold_entity: Btc public key. Such as `0x043858204f15d385da76fcbdf019debde624689e296c5ac53f6437491528857617691fe85c5c529b692bd75e361a9d0995dbd3e20a81e949642dfb74095520d981`.

**Get your own public key**

~~~sh
curl --location --request GET 'https://coming-server-v2.coming.chat/v1/accounts/getPublicKeyV2/{Cid}' \
--header 'Authorization: Basic NGNjNWZmODEtY2IwYi00MzE4LTkyZjYtOWRkNjBiNzBjYTRhOlg1VStRUGp3OFBmOU00Ung0bXBHNlN1Yw=='
~~~

Replace the above **Cid** with your own **Cid**, and enter in the terminal to find your own public key, as shown in the figure below.Note that what is needed is the public key corresponding to signet

![](https://cdn.jsdelivr.net/gh/AAweidai/PictureBed@master/taproot/16401639828951640163982867.png)

# Reward distribution

After the renewal of the trust each month, the previous trust can apply to the Treasury for 43750 ksx to the trust multi-signature account. After the ksx is received, any member of the previous trust can distribute rewards through the interface shown in the figure below. 

![img](https://cdn.jsdelivr.net/gh/hacpy/PictureBed@master/Document/16400549742281640054974219.png)

- sessionNum: The id of the previous trust.
