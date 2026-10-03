import { Button } from "@mantine/core";
import classes from "./ButtonInterval.module.css";

export type ButtonIntervalProps = {
  intervals: number[];
  disabled?: boolean;
  prefix: string;
  color: string;
  OnClick: (interval: number) => void;
};

export function ButtonInterval({ color, prefix, OnClick, intervals, disabled }: ButtonIntervalProps) {
  return (
    <>
      {intervals.map((interval) => (
        <Button key={interval} onClick={() => OnClick(interval)} variant="filled" color={color} className={classes.button} disabled={disabled}>
          {prefix}
          {interval}
        </Button>
      ))}
    </>
  );
}
