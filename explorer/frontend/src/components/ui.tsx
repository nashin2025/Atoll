import * as React from 'react';
import { cn } from './utils';

export interface SlotProps {
  children?: React.ReactNode;
  className?: string;
}

export function Slot({ children, className }: SlotProps) {
  return React.createElement('div', { className: cn(className) }, children);
}

// Shadcn-style base component that accepts className and merges with cn()
export interface BaseProps extends SlotProps {
  asChild?: boolean;
}

export function BaseComponent({
  asChild = false,
  className,
  children,
  ...props
}: BaseProps & Record<string, unknown>) {
  const Comp = asChild ? Slot : 'div';
  return (
    <Comp className={cn(className)} {...(props as Record<string, unknown>)}>{children}</Comp>
  );
}
