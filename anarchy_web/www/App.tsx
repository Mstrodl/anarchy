import "./App.css";
import React from "react";
import {useCallback, useState, useEffect, useRef} from "react";
import MonacoEditor from "react-monaco-editor";
import {monaco} from "react-monaco-editor";
import {ChangeHandler, EditorDidMount} from "react-monaco-editor";
import {
  WebError,
  WorkerToPageMessage,
  PageToWorkerMessage,
  getSavedCode,
} from "./workerTypes";
import {worker} from "./workerRef";

function isDeepEqual(a: any, b: any) {
  // if the number of keys is different, they are different
  if (Object.keys(a).length !== Object.keys(b).length) {
    return false;
  }

  for (const key in a) {
    const valueA = a[key];
    const valueB = b[key];
    // If the value is an object, check if they're different objects
    // If it isn't, uses !== to check
    if (
      (valueA instanceof Object && !isDeepEqual(valueA, valueB)) ||
      (!(valueA instanceof Object) && valueA !== valueB)
    ) {
      return false;
    }
  }
  return true;
}

const WIDTH = 100;
const HEIGHT = 100;

let lastRender: ArrayBuffer | null = null;
const SHARED_BUFFER = new Uint8ClampedArray(100 * 100 * 4);

export function App() {
  const canvasRef = useRef(null as HTMLCanvasElement | null);
  const canvasContextRef = useRef(null as CanvasRenderingContext2D | null);
  const [runtimeError, setRuntimeError] = useState(null as WebError | null);
  const [parseError, setParseError] = useState(null as WebError | null);
  useEffect(() => {
    if (canvasRef.current) {
      canvasContextRef.current = canvasRef.current.getContext("2d");
    }
  }, [canvasRef.current]);

  useEffect(() => {
    const onBlur = () => {
      sendMessage({type: "renderControl", running: false});
    };
    const onFocus = () => {
      sendMessage({type: "renderControl", running: true});
    };
    window.addEventListener("blur", onBlur);
    window.addEventListener("focus", onFocus);
    return () => {
      window.removeEventListener("blur", onBlur);
      window.removeEventListener("focus", onFocus);
    };
  }, []);

  useEffect(() => {
    const cb = (event: MessageEvent<WorkerToPageMessage>) => {
      const data = event.data;
      if (data.type == "draw") {
        const shouldSchedule = lastRender === null;
        lastRender = data.data;
        if (shouldSchedule) {
          requestAnimationFrame(() => {
            if (lastRender === null) {
              return;
            }
            SHARED_BUFFER.set(new Uint8ClampedArray(lastRender));
            canvasContextRef.current!.putImageData(
              new ImageData(SHARED_BUFFER, WIDTH, HEIGHT),
              0,
              0,
            );
            lastRender = null;
          });
        }
      } else if (data.type == "runtimeError") {
        setRuntimeError((runtimeError) => {
          if (runtimeError === null || data.error === null) {
            return data.error;
          }
          return runtimeError;
        });
      } else if (data.type == "parseError") {
        setParseError(data.error);
      }
    };
    worker.addEventListener("message", cb);
    return () => {
      worker.removeEventListener("message", cb);
    };
  }, []);

  return (
    <div className="editorBlock">
      <Editor runtimeError={runtimeError} parseError={parseError} />
      <div className="canvasBlock">
        <div className="canvasWrapper">
          <canvas width={WIDTH} height={HEIGHT} ref={canvasRef} />
        </div>
      </div>
    </div>
  );
}

function sendMessage(message: PageToWorkerMessage) {
  worker.postMessage(message);
}

function Editor({
  runtimeError,
  parseError,
}: {
  runtimeError: WebError | null;
  parseError: WebError | null;
}) {
  useEffect(() => {
    const cb = () => {
      if (monacoRef.current) {
        monacoRef.current.layout();
      }
    };
    window.addEventListener("resize", cb);
    cb();
    return () => {
      window.removeEventListener("resize", cb);
    };
  }, []);
  const monacoRef = useRef(null as null | monaco.editor.IStandaloneCodeEditor);
  const editorDidMount: EditorDidMount = useCallback((editor) => {
    monacoRef.current = editor;
    editor.layout();
    editor.focus();
  }, []);
  const [code, setCode] = useState(null as string | null);
  useEffect(() => {
    getSavedCode().then((code) => {
      setCode(code);
    });
  }, []);

  const onChange: ChangeHandler = useCallback((newValue: string) => {
    setCode(newValue);
    sendMessage({type: "parse", code: newValue});
  }, []);

  const pickedError = parseError || runtimeError;
  const decorations = useRef(
    null as monaco.editor.IEditorDecorationsCollection | null,
  );
  useEffect(() => {
    if (decorations.current) {
      decorations.current.clear();
    }
    const editor = monacoRef.current;
    if (!pickedError || !editor || pickedError.location == "None") {
      return;
    }

    const location = pickedError.location;
    const range =
      "Span" in location
        ? {
            startLineNumber: location.Span[0][0],
            startColumn: location.Span[0][1],
            endLineNumber: location.Span[1][0],
            endColumn: location.Span[1][1],
          }
        : {
            startLineNumber: location.Pos[0],
            startColumn: location.Pos[1],
            endLineNumber: location.Pos[0],
            endColumn: location.Pos[1] + 1,
          };

    decorations.current = editor.createDecorationsCollection([
      {
        options: {
          //isWholeLine: "Pos" in location,
          hoverMessage: {value: pickedError.message},
          className: "squiggly-error",
        },
        range,
      },
    ]);
  }, [pickedError]);
  if (code === null) {
    return null;
  }
  return (
    <div className="editor">
      <MonacoEditor
        width="100%"
        height="100%"
        language="anarchy"
        theme="vs-dark"
        value={code}
        onChange={onChange}
        editorDidMount={editorDidMount}
      />
      {pickedError && pickedError.location == "None" && (
        <div>{pickedError.message}</div>
      )}
    </div>
  );
}
