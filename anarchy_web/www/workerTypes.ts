import {get} from "idb-keyval";
import DEFAULT_CODE from "./input.anarchy";

export type PageToWorkerMessage =
  | {
      type: "parse";
      code: string;
    }
  | {
      type: "renderControl";
      running: boolean;
    };

export type WebError = {
  location:
    | {Span: [[number, number], [number, number]]}
    | {Pos: [number, number]}
    | "None";
  message: string;
  error_type: "Runtime" | "Parser";
};

export type WorkerToPageMessage =
  | {
      type: "draw";
      data: ArrayBuffer;
    }
  | {
      type: "parseError";
      error: WebError | null;
    }
  | {
      type: "runtimeError";
      error: WebError | null;
    }
  | {type: "parsed"; code: string};

export async function getSavedCode(): Promise<string> {
  return (
    (await get("saved-code")) || DEFAULT_CODE
  );
}
