import React from "react";
import ReactDOM from "react-dom/client";
import { GoldOverlayApp } from "./components/GoldOverlayApp";
import "./overlay.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <GoldOverlayApp />
  </React.StrictMode>
);
