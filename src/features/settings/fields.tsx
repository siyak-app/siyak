import { cn } from "cn";
import { CheckIcon, ChevronDownIcon } from "lucide-react";
import {
  type CSSProperties,
  type KeyboardEvent,
  type ReactNode,
  useId,
  useRef,
  useState,
} from "react";

import { Checkbox } from "@/components/ui/checkbox";

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
}: {
  legend: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
  hint?: ReactNode;
}) {
  const name = useId();
  const hintId = useId();
  return (
    <fieldset aria-describedby={hint ? hintId : undefined} className="grid gap-1.5">
      <legend className="mb-1 text-sm font-medium">{legend}</legend>
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
  disabled = false,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  hint?: ReactNode;
  /** Cannot be changed right now, e.g. a limit is reached. */
  disabled?: boolean;
}) {
  const id = useId();
  const hintId = useId();
  return (
    <div className="grid gap-1">
      <div className="flex items-center gap-2">
        <Checkbox
          id={id}
          checked={checked}
          disabled={disabled}
          aria-describedby={hint ? hintId : undefined}
          onCheckedChange={(value) => {
            onChange(value === true);
          }}
        />
        <label
          htmlFor={id}
          className={disabled ? "text-sm text-muted-foreground" : "text-sm font-medium"}
        >
          {label}
        </label>
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

/** A titled group of settings in a soft card. */
export function SettingsSection({
  title,
  description,
  action,
  children,
}: {
  title: string;
  description?: ReactNode;
  /** A control beside the title, e.g. a reset button. */
  action?: ReactNode;
  children: ReactNode;
}) {
  const id = useId();
  return (
    <section aria-labelledby={id} className="grid gap-3">
      <div className="flex items-end justify-between gap-4">
        <div className="grid gap-0.5">
          <h3 id={id} className="text-sm font-semibold">
            {title}
          </h3>
          {description && <p className="text-xs text-muted-foreground">{description}</p>}
        </div>
        {action}
      </div>
      <div className="grid gap-4 rounded-xl border bg-card p-4">{children}</div>
    </section>
  );
}

/** One option of a {@link SelectCard}: its label and an optional preview. */
export interface CardOption<T extends string> {
  value: T;
  label: string;
  /** Drawn before the label, e.g. colour swatches. */
  preview?: ReactNode;
  /** Applied to the label, e.g. the typeface it names. */
  style?: CSSProperties;
}

/**
 * One choice out of many as a long, thin card: it shows the current option
 * and opens the list of options under itself when pressed. The options are a
 * radio group (arrow keys move, Enter or Space chooses); choosing closes it.
 */
export function SelectCard<T extends string>({
  label,
  hint,
  value,
  options,
  onChange,
  columns = 1,
}: {
  label: string;
  hint?: ReactNode;
  value: T;
  options: readonly CardOption<T>[];
  onChange: (value: T) => void;
  /** Columns of the open list on wide dialogs. */
  columns?: 1 | 2;
}) {
  const [open, setOpen] = useState(false);
  const labelId = useId();
  const listId = useId();
  const hintId = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const radios = useRef<(HTMLButtonElement | null)[]>([]);
  const current = options.find((option) => option.value === value) ?? options[0];

  const choose = (next: T) => {
    onChange(next);
    setOpen(false);
    trigger.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = radios.current.indexOf(document.activeElement as HTMLButtonElement);
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[event.key];
    if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
      trigger.current?.focus();
    } else if (step !== undefined && index >= 0) {
      event.preventDefault();
      radios.current[(index + step + options.length) % options.length]?.focus();
    }
  };

  return (
    <div className="grid gap-1.5">
      <span id={labelId} className="text-sm font-medium">
        {label}
      </span>
      <button
        ref={trigger}
        type="button"
        aria-expanded={open}
        aria-controls={listId}
        aria-labelledby={labelId}
        aria-describedby={hint ? hintId : undefined}
        onClick={() => {
          setOpen(!open);
        }}
        className="flex h-11 w-full items-center gap-3 rounded-lg border bg-background px-3 text-start text-sm transition-colors outline-none hover:border-primary/40 hover:bg-muted/40 focus-visible:ring-2 focus-visible:ring-ring aria-expanded:border-primary/50"
      >
        {current?.preview}
        <span className="min-w-0 flex-1 truncate" style={current?.style}>
          {current?.label}
        </span>
        <ChevronDownIcon
          aria-hidden
          className={cn(
            "size-4 shrink-0 text-muted-foreground transition-transform duration-200",
            open && "rotate-180",
          )}
        />
      </button>
      {open && (
        <div
          id={listId}
          role="radiogroup"
          aria-labelledby={labelId}
          onKeyDown={onKeyDown}
          className={cn(
            "grid animate-in gap-1 rounded-lg border bg-popover p-1 shadow-sm duration-150 fade-in-0 slide-in-from-top-1",
            columns === 2 && "sm:grid-cols-2",
          )}
        >
          {options.map((option, index) => {
            const checked = option.value === value;
            return (
              <button
                key={option.value}
                ref={(element) => {
                  radios.current[index] = element;
                }}
                type="button"
                role="radio"
                aria-checked={checked}
                // Focus moves into the list the user just opened.
                autoFocus={checked}
                tabIndex={checked ? 0 : -1}
                onClick={() => {
                  choose(option.value);
                }}
                className="flex min-h-9 items-center gap-3 rounded-md px-2.5 py-1.5 text-start text-sm transition-colors outline-none hover:bg-muted focus-visible:bg-muted focus-visible:ring-2 focus-visible:ring-ring aria-checked:bg-primary/10 aria-checked:font-medium"
              >
                {option.preview}
                <span className="min-w-0 flex-1" style={option.style}>
                  {option.label}
                </span>
                {checked && <CheckIcon aria-hidden className="size-4 shrink-0 text-primary" />}
              </button>
            );
          })}
        </div>
      )}
      {hint && <Hint id={hintId}>{hint}</Hint>}
    </div>
  );
}

/** One choice out of two or three short ones, as a segmented control. */
export function SegmentedField<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  const labelId = useId();
  return (
    <div className="flex flex-wrap items-center justify-between gap-2">
      <span id={labelId} className="text-sm font-medium">
        {label}
      </span>
      <div
        role="radiogroup"
        aria-labelledby={labelId}
        className="flex gap-0.5 rounded-lg border bg-muted/60 p-0.5"
      >
        {options.map((option) => {
          const checked = option.value === value;
          return (
            <button
              key={option.value}
              type="button"
              role="radio"
              aria-checked={checked}
              onClick={() => {
                onChange(option.value);
              }}
              className={cn(
                "h-7 rounded-md px-3 text-xs font-medium transition-all outline-none focus-visible:ring-2 focus-visible:ring-ring",
                checked
                  ? "bg-background text-foreground shadow-sm"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {option.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
