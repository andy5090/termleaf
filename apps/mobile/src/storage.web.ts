import { parseLibrary, type Storage } from "@termleaf/notebook";

const storageKey = "termleaf.library.v1";

/** Browser preview storage. Native builds use the document-directory adapter. */
export const mobileStorage: Storage = {
  async read() {
    const value = globalThis.localStorage.getItem(storageKey);
    if (value !== null) parseLibrary(value);
    return value;
  },
  async write(contents) {
    parseLibrary(contents);
    globalThis.localStorage.setItem(storageKey, contents);
  },
};
