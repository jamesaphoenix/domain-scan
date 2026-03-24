import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ToastProvider } from "./hooks/useToast";
import { ToastContainer } from "./components/ToastContainer";
import "@xyflow/react/dist/style.css";
import "./styles.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ToastProvider>
      <App />
      <ToastContainer />
    </ToastProvider>
  </React.StrictMode>,
);
