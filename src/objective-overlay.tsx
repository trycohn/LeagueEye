import React from "react";
import ReactDOM from "react-dom/client";
import { ObjectiveOverlayApp } from "./components/ObjectiveOverlayApp";
import "./overlay.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ObjectiveOverlayApp />
  </React.StrictMode>
);
