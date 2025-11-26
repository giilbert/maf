"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
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

// src/store.ts
var store_exports = {};
__export(store_exports, {
  Store: () => Store
});
module.exports = __toCommonJS(store_exports);
var import_emittery = __toESM(require("emittery"), 1);
var Store = class extends import_emittery.default {
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
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  Store
});
