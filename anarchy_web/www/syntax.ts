import {monaco} from "react-monaco-editor";

monaco.languages.register({
  id: "anarchy",
});

monaco.languages.registerTokensProviderFactory("anarchy", {
  create: async (): Promise<monaco.languages.IMonarchLanguage> => {
    return language;
  },
});
monaco.languages.onLanguageEncountered("anarchy", async () => {
  monaco.languages.setLanguageConfiguration("anarchy", {});
});

const language: monaco.languages.IMonarchLanguage = {
  // Set defaultToken to invalid to see what you do not tokenize yet
  defaultToken: "invalid",
  tokenPostfix: ".anarchy",

  keywords: ["else", "if", "function", "return"],

  typeKeywords: [],

  operators: [
    "<=",
    ">=",
    "==",
    "!=",
    "+",
    "-",
    "**",
    "*",
    "/",
    "%",
    "<<",
    ">>",
    "&",
    "|",
    "^",
    "!",
    "&&",
    "||",
    "=",
  ],

  // we include these common regular expressions
  symbols: /[=><!~?:&|+\-*\/\^%]+/,
  digits: /\d+(\.\d+)?/,

  // The main tokenizer for our languages
  tokenizer: {
    root: [[/[{}]/, "delimiter.bracket"], {include: "common"}],

    common: [
      // identifiers and keywords
      [
        /[A-Za-z_$][\w$]*/,
        {
          cases: {
            "@typeKeywords": "keyword",
            "@keywords": "keyword",
            "@default": "identifier",
          },
        },
      ],

      // whitespace
      {include: "@whitespace"},

      // delimiters and operators
      [/[()\[\]]/, "@brackets"],
      [/[<>](?!@symbols)/, "@brackets"],
      [
        /@symbols/,
        {
          cases: {
            "@operators": "delimiter",
            "@default": "",
          },
        },
      ],

      // numbers
      [/(@digits)/, "number"],

      // delimiter: after number because of .\d floats
      [/[;,.]/, "delimiter"],

      // strings
      [/"([^"\\]|\\.)*$/, "string.invalid"], // non-teminated string
      [/"/, "string", "@string_double"],
    ],

    whitespace: [
      [/[ \t\r\n]+/, ""],
      [/\/\*/, "comment", "@comment"],
      [/\/\/.*$/, "comment"],
    ],

    comment: [
      [/[^\/*]+/, "comment"],
      [/\*\//, "comment", "@pop"],
      [/[\/*]/, "comment"],
    ],

    string_double: [
      [/[^\\"]+/, "string"],
      [/"/, "string", "@pop"],
    ],

    bracketCounting: [
      [/\{/, "delimiter.bracket", "@bracketCounting"],
      [/\}/, "delimiter.bracket", "@pop"],
      {include: "common"},
    ],
  },
};
