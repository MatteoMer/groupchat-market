# Groupchat Market: Prediction markets for your groupchat

Built during [Frontiers](https://frontiers.paradigm.xyz/)

**Turn your chats into prediction markets**

---

### Example

```
Alice: "I bet John will be late to the meeting again"
Bob: /new Will John arrive on time to the 3pm meeting?
Bot: ✅ Market #1 created: Will John arrive on time to the 3pm meeting?
Alice: /bet 1 no 500
Bob: /bet 1 yes 100
Charlie: /bet 1 no 200

[Later at 3:05pm]
John: "Sorry guys, traffic was terrible"
Alice: /solve 1 [replying to John's message]
Bot: ✅ MARKET RESOLVED: NO wins
     🤖 AI Analysis: "John apologized for being late, confirming he did not arrive on time"
     💰 Payouts: Alice +750, Charlie +300
```

---

### Architecture

Everything: bot, server and the contract are written in Rust. The contract is a vApp proven on a zkVM. 

This proof is used for settlement on [Hyli](https://hyli.org/), a blockchain where every app is a vApp and where the execution is offchain, the consensus is only verifying the proof

## License

MIT
