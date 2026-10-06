import type { ReactNode } from "react";

export default function Collapse({ open, pad, children }: { open: boolean; pad?: boolean; children: ReactNode }) {
  return (
    <div className={"collapse" + (open ? " open" : "")} inert={!open}>
      <div className="collapse-in">{pad ? <div className="pad-b">{children}</div> : children}</div>
    </div>
  );
}
