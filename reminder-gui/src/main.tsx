import React from "react";
import ReactDOM from "react-dom/client";
import { OverlayProvider } from "@pikoloo/darwin-ui";
import App from "./App";
import "@pikoloo/darwin-ui/styles";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <OverlayProvider>
      <App />
    </OverlayProvider>
  </React.StrictMode>
);
