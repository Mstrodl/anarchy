import {get} from "idb-keyval";

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
    (await get("saved-code")) ||
    `time=time/250;
r=(y*time)&255;
g=(x*time)&255;
b=(cos(time/20)*128 + 128);

if ((sin(x/10+time)*50+50)|0 == y) {
  r=255;
  g=255;
  b=255;
}`
  );
}
