# cTrader Protobuf Definitions

## ⚠️ Important Notice

The `ctrader.proto` file in this directory is a **STUB** for compilation purposes.

For **production use**, you MUST replace it with the official cTrader Open API proto files.

## 📥 Download Official Proto Files

1. **GitHub Repository**: https://github.com/spotware/OpenApiProto
2. **Official Documentation**: https://help.ctrader.com/open-api/

## 🔧 Installation

```bash
# Clone the official proto definitions
git clone https://github.com/spotware/OpenApiProto.git

# Copy to proto directory
cp OpenApiProto/*.proto /home/julien/Documents/palm-oil-bot/proto/

# Rebuild
cargo clean
cargo build
```

## 📋 Required Proto Files

For full functionality, you need:

- `OpenApiCommonMessages.proto` - Common message types
- `OpenApiMessages.proto` - Main API messages
- `OpenApiModelMessages.proto` - Data models

## 🧪 Current Stub Coverage

The stub provides minimal definitions for:

- ✅ Authentication (ApplicationAuthReq/Res, AccountAuthReq/Res)
- ✅ Market data (SubscribeSpotsReq, SpotEvent)
- ✅ Orders (NewOrderReq, ExecutionEvent)
- ✅ Basic enums (TradeSide, OrderType, OrderStatus)

**Missing from stub** (in official protos):
- Historical data requests
- Symbol information
- Account details
- Full order management
- And 50+ other message types

## 🚀 Production Deployment

Before deploying to production:

1. [ ] Download official proto files
2. [ ] Update `build.rs` if needed
3. [ ] Rebuild project: `cargo build --release`
4. [ ] Test all cTrader API functionality
5. [ ] Verify Protobuf message serialization

## 📚 Resources

- **cTrader API Docs**: https://help.ctrader.com/open-api/
- **Proto3 Language Guide**: https://protobuf.dev/programming-guides/proto3/
- **Prost (Rust)**: https://github.com/tokio-rs/prost
