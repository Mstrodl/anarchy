import {set} from "idb-keyval";
import {
  getSavedCode,
  WebError,
  PageToWorkerMessage,
  WorkerToPageMessage,
} from "./workerTypes";

let anarchy: typeof import("anarchy_web") | null = null;

async function start() {
  anarchy = await import("anarchy_web");
  anarchy.init();
  const code = await getSavedCode();
  anarchy.parse(code);
  sendMessage({type: "parsed", code});
  run();
}

let runInterval = null as null | number;
function run() {
  runInterval = setInterval(() => {
    const imageBuffer = new ArrayBuffer(HEIGHT * WIDTH * 4);
    const buffer = new Uint8Array(imageBuffer);
    buffer.fill(255);
    try {
      anarchy?.execute(buffer, WIDTH, HEIGHT, Date.now() - time, random);
    } catch (err) {
      if (err && typeof err == "object" && (err as WebError).error_type) {
        const newError = err as WebError;
        sendMessage({type: "runtimeError", error: newError});
      }
      return;
    }

    sendMessage({
      type: "draw",
      data: imageBuffer,
    });
  });
}

function sendMessage(data: WorkerToPageMessage) {
  self.postMessage(data);
}

start();

const WIDTH = 100;
const HEIGHT = 100;
// const IMAGE_BUFFER = new SharedArrayBuffer(WIDTH * HEIGHT * 4);
// () => {
//   const array = new Uint8ClampedArray(IMAGE_BUFFER);
//   array.fill(255);
// };

let time = Date.now();
let random = Math.random();

self.addEventListener("message", (event: MessageEvent<PageToWorkerMessage>) => {
  handlePageMessage(event.data);
});

function parse(code: string) {
  time = Date.now();
  random = Math.random();
  sendMessage({type: "runtimeError", error: null});
  try {
    anarchy?.parse(code);
  } catch (err) {
    if (err && typeof err == "object" && (err as WebError).error_type) {
      const newError = err as WebError;
      sendMessage({type: "parseError", error: newError});
    }
    return;
  }
  sendMessage({type: "parseError", error: null});
  sendMessage({type: "parsed", code: code});
}

async function handlePageMessage(data: PageToWorkerMessage) {
  if (data.type == "parse") {
    await set("saved-code", data.code);
    parse(data.code);
  } else if (data.type == "renderControl") {
    if (data.running) {
      if (!runInterval) {
        run();
      }
    } else {
      if (runInterval) {
        clearInterval(runInterval);
        runInterval = null;
      }
    }
  }
}
