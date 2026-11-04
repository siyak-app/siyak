import type { LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";

/** How long the second press stays possible. */
const CONFIRM_MS = 4000;

/**
 * A button for what cannot be undone: the first press asks, and a second press
 * within a few seconds does it.
 */
export function ConfirmButton({
  icon: Icon,
  label,
  confirmLabel,
  onConfirm,
  size = "xs",
}: {
  icon?: LucideIcon;
  label: string;
  confirmLabel: string;
  onConfirm: () => void;
  size?: "xs" | "sm";
}) {
  const [asking, setAsking] = useState(false);
  useEffect(() => {
    if (!asking) return;
    const timer = setTimeout(() => {
      setAsking(false);
    }, CONFIRM_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [asking]);
  return (
    <Button
      variant={asking ? "destructive" : "ghost"}
      size={size}
      className={asking ? undefined : "text-muted-foreground"}
      onClick={() => {
        if (asking) {
          setAsking(false);
          onConfirm();
        } else {
          setAsking(true);
        }
      }}
    >
      {Icon && <Icon aria-hidden />}
      {asking ? confirmLabel : label}
    </Button>
  );
}
