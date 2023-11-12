import {App} from "./App";
import ReactDOM from "react-dom";
import React from "react";
import "./syntax";

function start(anarchy: typeof import("anarchy_web")) {
  anarchy.init();
  anarchy.parse(`r=time&255;
g=time&255;
b=time&255;
`);
  console.log("All modules loaded");
  //anarchy.my_exported_rust_function();

  ReactDOM.render(
    React.createElement(App, {anarchy}),
    document.getElementById("app-root"),
  );
}

async function load() {
  start(await import("anarchy_web"));
}

load();
