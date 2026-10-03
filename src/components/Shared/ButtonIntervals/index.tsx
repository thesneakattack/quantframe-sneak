import { ButtonInterval } from "../ButtonInterval";

export type ButtonIntervalsProps = {
  intervals: number[];
  disabled?: boolean;
  minimum_price?: number;
  OnClick: (interval: number) => void;
};

export function ButtonIntervals({ intervals, minimum_price, OnClick, disabled }: ButtonIntervalsProps) {
  return (
    <>
      <ButtonInterval
        disabled={disabled}
        color="red.7"
        intervals={intervals}
        prefix="-"
        OnClick={async (int) => {
          minimum_price = minimum_price || 0;
          if (minimum_price - int < 0) return;
          OnClick(minimum_price - int);
        }}
      />
      <ButtonInterval
        disabled={disabled}
        color="green.7"
        intervals={intervals}
        prefix="+"
        OnClick={async (int) => {
          minimum_price = minimum_price || 0;
          OnClick(minimum_price + int);
        }}
      />
    </>
  );
}
