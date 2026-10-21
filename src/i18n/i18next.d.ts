import type en from "./en.json";

// Makes `t()` reject unknown keys at compile time. English is the source of
// truth; a test checks that tr.json has exactly the same keys.
declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    resources: { translation: typeof en };
  }
}
