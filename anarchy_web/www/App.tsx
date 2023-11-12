import React from "react";
import {useCallback, useState, useEffect, useRef} from "react";
import MonacoEditor from "react-monaco-editor";
import {monaco} from "react-monaco-editor";
import {ChangeHandler, EditorDidMount} from "react-monaco-editor";

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

type WebError = {
  location:
    | {Span: [[number, number], [number, number]]}
    | {Pos: [number, number]}
    | "None";
  message: string;
  error_type: "Runtime" | "Parser";
};

const WIDTH = 100;
const HEIGHT = 100;
export function App({anarchy}: {anarchy: typeof import("anarchy_web")}) {
  // anarchy.parse("a = 5;");
  //
  // let random = Math.random();
  // for (let time = 0; time < 10; ++time) {
  //   anarchy.execute(image, WIDTH, HEIGHT, time, random);
  // }
  const canvasRef = useRef(null as HTMLCanvasElement | null);
  const canvasContextRef = useRef(null as CanvasRenderingContext2D | null);
  const imageRef = useRef(null as unknown as Uint8Array);
  if (!imageRef.current) {
    imageRef.current = new Uint8Array(WIDTH * HEIGHT * 4);
    imageRef.current.fill(255);
  }
  useEffect(() => {
    if (canvasRef.current) {
      canvasContextRef.current = canvasRef.current.getContext("2d");
    }
  }, [canvasRef.current]);
  const [error, setError] = useState(null as WebError | null);
  useEffect(() => {
    let time = 0;
    const random = Math.random();
    let finished = false;
    function renderFrame() {
      requestAnimationFrame(() => {
        // console.time("One frame");
        try {
          anarchy.execute(imageRef.current, WIDTH, HEIGHT, time++, random);
        } catch (err) {
          if (err && typeof err == "object" && (err as WebError).error_type) {
            const newError = err as WebError;
            //console.log(newError);
            setError((error) => {
              if (error && isDeepEqual(error, newError)) {
                return error;
              } else {
                return newError;
              }
            });
          }
          if (!finished) {
            renderFrame();
          }
          return;
        }
        setError(null);
        //console.log("Chom", imageRef.current);
        canvasContextRef.current!.putImageData(
          new ImageData(
            new Uint8ClampedArray(imageRef.current.buffer),
            WIDTH,
            HEIGHT,
          ),
          0,
          0,
        );
        // console.timeEnd("One frame");
        if (!finished) {
          renderFrame();
        }
      });
    }
    if (canvasContextRef.current) {
      renderFrame();
      return () => {
        finished = true;
      };
    }
  }, [canvasContextRef.current]);

  return (
    <div>
      <canvas width={WIDTH} height={HEIGHT} ref={canvasRef} />
      <Editor anarchy={anarchy} runtimeError={error} />
    </div>
  );
}

function Editor({
  anarchy,
  runtimeError,
}: {
  anarchy: typeof import("anarchy_web");
  runtimeError: WebError | null;
}) {
  const monacoRef = useRef(null as null | monaco.editor.IStandaloneCodeEditor);
  const editorDidMount: EditorDidMount = useCallback((editor) => {
    monacoRef.current = editor;
    editor.focus();
  }, []);
  const [code, setCode] = useState(`r=time&255;
g=time&255;
b=time&255;`);
  const onChange: ChangeHandler = useCallback((newValue: string) => {
    setCode(newValue);
  }, []);
  const [error, setError] = useState(null as WebError | null);
  useEffect(() => {
    try {
      anarchy.parse(code);
    } catch (err) {
      if (err && typeof err == "object" && (err as WebError).error_type) {
        const newError = err as WebError;
        console.log(newError);
        setError((error) => {
          if (error && isDeepEqual(error, newError)) {
            return error;
          } else {
            return newError;
          }
        });
      }
      return;
    }
    setError(null);
  }, [code]);
  const pickedError = error || runtimeError;
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
  return (
    <div>
      <MonacoEditor
        width="800"
        height="600"
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
