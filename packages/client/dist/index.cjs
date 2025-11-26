"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getOwnPropSymbols = Object.getOwnPropertySymbols;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __propIsEnum = Object.prototype.propertyIsEnumerable;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __spreadValues = (a, b) => {
  for (var prop in b || (b = {}))
    if (__hasOwnProp.call(b, prop))
      __defNormalProp(a, prop, b[prop]);
  if (__getOwnPropSymbols)
    for (var prop of __getOwnPropSymbols(b)) {
      if (__propIsEnum.call(b, prop))
        __defNormalProp(a, prop, b[prop]);
    }
  return a;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);
var __async = (__this, __arguments, generator) => {
  return new Promise((resolve, reject) => {
    var fulfilled = (value) => {
      try {
        step(generator.next(value));
      } catch (e) {
        reject(e);
      }
    };
    var rejected = (value) => {
      try {
        step(generator.throw(value));
      } catch (e) {
        reject(e);
      }
    };
    var step = (x) => x.done ? resolve(x.value) : Promise.resolve(x.value).then(fulfilled, rejected);
    step((generator = generator.apply(__this, __arguments)).next());
  });
};

// src/index.ts
var index_exports = {};
__export(index_exports, {
  DEFAULT_DEV_SERVER_URL: () => DEFAULT_DEV_SERVER_URL,
  MafClient: () => MafClient,
  Store: () => Store,
  TypedMafClient: () => TypedMafClient
});
module.exports = __toCommonJS(index_exports);

// src/client.ts
var import_emittery3 = __toESM(require("emittery"), 1);

// src/channel.ts
var import_emittery = __toESM(require("emittery"), 1);
var Channel = class extends import_emittery.default {
  constructor(client, name) {
    super();
    this.client = client;
    this.name = name;
  }
  send(message) {
    this.client.send({
      type: "ChannelSend",
      data: {
        channel: this.name,
        data: message
      }
    });
  }
};

// src/store.ts
var import_emittery2 = __toESM(require("emittery"), 1);
var Store = class extends import_emittery2.default {
  constructor(client, name, options) {
    var _a;
    super();
    this._hasInit = false;
    this._data = null;
    const storeInit = (_a = options == null ? void 0 : options.default) != null ? _a : null;
    this.client = client;
    this.name = name;
    if (storeInit) {
      this._data = storeInit;
      this._hasInit = true;
    }
    this.on("change", (data) => {
      this._data = data;
    });
    this.init = new Promise((resolve) => {
      if (this._hasInit) return resolve();
      const unsubscribe = this.on("change", (data) => {
        if ((options == null ? void 0 : options.hasInit) && !options.hasInit(data)) return;
        this._hasInit = true;
        unsubscribe();
        resolve();
      });
    });
  }
  /**
   * Gets the data currently inside the store. This getter is guaranteed to
   * result in a non-null `T`.
   *
   * If the store has not been initialized (see `this.init`), this getter method
   * will error. `this.get` is the fallible version of this, returning null
   * if the store has not been initialized.
   */
  get data() {
    if (!this._hasInit)
      throw new Error("Store has not been initialized with data.");
    return this._data;
  }
  get() {
    return this._data;
  }
  get hasInit() {
    return this._hasInit;
  }
};

// src/client.ts
var DEFAULT_DEV_SERVER_URL = "http://localhost:1147";
var MafUntypedBaseClient = class extends import_emittery3.default {
  constructor(options) {
    super();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    this._channels = {};
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    this._stores = {};
    this._storeData = {};
    this._rpcId = 0;
    this._rpcCalls = /* @__PURE__ */ new Map();
    this._cleanups = [];
    let url;
    if (options.server === "dev") {
      url = new URL(DEFAULT_DEV_SERVER_URL);
      url.pathname = "@/_/_";
    } else if (options.server.type === "dev") {
      url = new URL(options.server.url || DEFAULT_DEV_SERVER_URL);
      url.pathname = "@/_/_";
    } else if (options.server.type === "platform") {
      url = new URL(options.server.url);
      url.pathname = `@/${options.server.app}`;
    } else {
      throw new Error("Invalid server options");
    }
    this.url = url;
  }
  get ws() {
    if (!this._ws) throw new Error("WebSocket is not connected");
    return this._ws;
  }
  get sessionInfo() {
    if (!this._sessionInfo) throw new Error("Session info is not available");
    return this._sessionInfo;
  }
  connect() {
    return __async(this, arguments, function* (options = { type: "default" }) {
      const connectionUrl = new URL(this.url);
      if (options.type === "room") {
        connectionUrl.pathname += `/${options.id}/connect`;
        connectionUrl.searchParams.set("secret", options.secret);
      } else if (options.type === "default") {
        connectionUrl.pathname += "/default/connect";
      } else {
        throw new Error("Invalid connection options");
      }
      const ws = new WebSocket(connectionUrl);
      this._ws = ws;
      yield new Promise((resolve, reject) => {
        ws.addEventListener("open", resolve, { once: true });
        ws.addEventListener("error", reject, { once: true });
      });
      ws.send(
        JSON.stringify({
          type: "Handshake",
          data: {
            auth: {
              username: "hello",
              session: "12345"
            }
          }
        })
      );
      const handshakeResponse = yield new Promise(
        (resolve, reject) => {
          ws.addEventListener(
            "message",
            (event) => {
              const { data, type } = JSON.parse(event.data);
              if (type === "Handshake") resolve(data);
            },
            { once: true }
          );
          ws.addEventListener("error", reject, { once: true });
        }
      );
      this._sessionInfo = handshakeResponse;
      this.emit("ready", handshakeResponse);
      const handleMessage = (event) => {
        if (typeof event.data === "string") {
          this.handleMessage(JSON.parse(event.data));
        } else {
          console.warn("Received non-string message:", event.data);
        }
      };
      ws.addEventListener("message", handleMessage);
      this._cleanups.push(() => {
        ws.removeEventListener("message", handleMessage);
      });
      ws.addEventListener(
        "close",
        () => {
          ws.removeEventListener("message", handleMessage);
          this.emit("close", void 0);
        },
        { once: true }
      );
      return handshakeResponse;
    });
  }
  disconnect() {
    if (this._ws) {
      if (this._ws.readyState === WebSocket.OPEN) {
        this._ws.close();
        for (const cleanup of this._cleanups) cleanup();
      } else if (this._ws.readyState === WebSocket.CONNECTING) {
        const wsRef = this._ws;
        this._ws.onopen = () => {
          wsRef.close();
          for (const cleanup of this._cleanups) cleanup();
        };
      }
      this._ws = void 0;
      this._sessionInfo = void 0;
    }
  }
  handleMessage(packet) {
    return __async(this, null, function* () {
      var _a, _b, _c, _d;
      if (packet.type === "ChannelSend") {
        const { channel, data } = packet.data;
        (_a = this._channels[channel]) == null ? void 0 : _a.emit("message", data);
      } else if (packet.type === "TypedRpcResponse") {
        const { id, result } = packet.data;
        (_b = this._rpcCalls.get(id)) == null ? void 0 : _b(result);
        this._rpcCalls.delete(id);
      } else if (packet.type === "ManyStoreUpdate") {
        for (const { store, data } of packet.data) {
          this._storeData[store] = data;
          (_c = this._stores[store]) == null ? void 0 : _c.emit("change", data);
        }
      } else if (packet.type === "StoreUpdate") {
        const { store, data } = packet.data;
        this._storeData[store] = data;
        (_d = this._stores[store]) == null ? void 0 : _d.emit("change", data);
      }
    });
  }
  channel(name) {
    if (!this._channels[name])
      this._channels[name] = new Channel(this, name);
    return this._channels[name];
  }
  send(message) {
    this.ws.send(JSON.stringify(message));
  }
  untypedRpc(method, ...params) {
    const id = this._rpcId++;
    this.send({
      type: "TypedRpcCall",
      data: {
        method,
        id,
        params: params.length === 1 ? params[0] : params
      }
    });
    return new Promise((resolve, reject) => {
      const MAX_RPC_CALLS = 5e3;
      if (this._rpcCalls.size > MAX_RPC_CALLS) {
        reject(
          new Error(`Maximum number of RPC calls exceeded (${MAX_RPC_CALLS})`)
        );
        return;
      }
      this._rpcCalls.set(id, (data) => {
        if (data instanceof Error) {
          reject(data);
        } else {
          resolve(data);
        }
      });
    });
  }
  untypedStore(name, options) {
    const data = this._storeData[name];
    if (!this._stores[name])
      this._stores[name] = new Store(this, name, __spreadValues({
        default: data
      }, options));
    return this._stores[name];
  }
};
var MafClient = class extends MafUntypedBaseClient {
  constructor(options) {
    super(options);
  }
  store(name, options) {
    return this.untypedStore(name, options);
  }
  rpc(method, ...params) {
    return this.untypedRpc(method, ...params);
  }
};

// src/typed.ts
var TypedMafClient = class extends MafUntypedBaseClient {
  constructor(options) {
    super(options);
  }
  store(name, options) {
    return super.untypedStore(name, options);
  }
  rpc(method, ...params) {
    return this.untypedRpc(method, ...params);
  }
};
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  DEFAULT_DEV_SERVER_URL,
  MafClient,
  Store,
  TypedMafClient
});
