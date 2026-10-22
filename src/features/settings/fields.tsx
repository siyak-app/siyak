import { type ReactNode, useId } from "react";
import { useTranslation } from "react-i18next";

import { Checkbox } from "@/components/ui/checkbox";

/** Marks a setting whose feature arrives in a later milestone. */
function LaterBadge() {
  const { t } = useTranslation();
  return (
    <span className="ms-2 rounded-full border px-2 py-0.5 text-xs font-normal text-muted-foreground">
      {t("settings.later_version")}
    </span>
  );
}

function Hint({ id, children }: { id: string; children: ReactNode }) {
  return (
    <p id={id} className="text-xs text-muted-foreground">
      {children}
    </p>
  );
}

/** One choice out of a few, as a group of radio buttons. */
export function RadioField<T extends string>({
  legend,
  value,
  options,
  onChange,
  hint,
  unavailable = false,
}: {
  legend: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
  hint?: ReactNode;
  /** The feature does not exist yet: shown, but disabled. */
  unavailable?: boolean;
}) {
  const name = useId();
  const hintId = useId();
  return (
    <fieldset
      disabled={unavailable}
      aria-describedby={hint ? hintId : undefined}
      className="grid gap-1.5"
    >
      <legend className="mb-1 text-sm font-medium">
        {legend}
        {unavailable && <LaterBadge />}
      </legend>
      {options.map((option) => (
        <label
          key={option.value}
          className="flex items-center gap-2 text-sm has-disabled:text-muted-foreground"
        >
          <input
            type="radio"
            name={name}
            value={option.value}
            checked={value === option.value}
            onChange={() => {
              onChange(option.value);
            }}
            className="size-4 accent-primary"
          />
          {option.label}
        </label>
      ))}
      {hint && <Hint id={hintId}>{hint}</Hint>}
    </fieldset>
  );
}

/** An on/off setting. */
export function CheckField({
  label,
  checked,
  onChange,
  hint,
  unavailable = false,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  hint?: ReactNode;
  unavailable?: boolean;
}) {
  const id = useId();
  const hintId = useId();
  return (
    <div className="grid gap-1">
      <div className="flex items-center gap-2">
        <Checkbox
          id={id}
          checked={checked}
          disabled={unavailable}
          aria-describedby={hint ? hintId : undefined}
          onCheckedChange={(value) => {
            onChange(value === true);
          }}
        />
        <label
          htmlFor={id}
          className={unavailable ? "text-sm text-muted-foreground" : "text-sm font-medium"}
        >
          {label}
        </label>
        {unavailable && <LaterBadge />}
      </div>
      {hint && <Hint id={hintId}>{hint}</Hint>}
    </div>
  );
}

/** A number within a range, as a slider with its current value. */
export function RangeField({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  /** The value as shown and announced, e.g. "28 px". */
  format: (value: number) => string;
  onChange: (value: number) => void;
}) {
  const id = useId();
  const text = format(value);
  return (
    <div className="grid gap-1">
      <div className="flex items-baseline justify-between gap-4 text-sm">
        <label htmlFor={id} className="font-medium">
          {label}
        </label>
        <output htmlFor={id} className="text-muted-foreground tabular-nums">
          {text}
        </output>
      </div>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        aria-valuetext={text}
        onChange={(event) => {
          onChange(Number(event.target.value));
        }}
        className="w-full accent-primary"
      />
    </div>
  );
}
