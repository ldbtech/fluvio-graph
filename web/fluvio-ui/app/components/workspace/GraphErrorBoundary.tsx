"use client";

import React, { type ReactNode } from "react";

type State = { hasError: boolean };

export class GraphErrorBoundary extends React.Component<{ children: ReactNode }, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(err: Error) {
    console.error("GraphCanvas error:", err);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-full min-h-[240px] flex-col items-center justify-center gap-3 bg-[#0a0612] px-6 text-center">
          <p className="max-w-md font-mono text-sm text-amber-100/90">
            The graph view stopped after an error (often the graph is very large). Refresh the page, or reduce Gmail
            sync limits and try again.
          </p>
          <button
            type="button"
            className="rounded-lg border border-cyan-400/40 px-4 py-2 font-mono text-xs text-cyan-100 hover:bg-cyan-500/10"
            onClick={() => this.setState({ hasError: false })}
          >
            Retry layout
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
