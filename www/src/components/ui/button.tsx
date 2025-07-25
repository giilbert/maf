import { cva, type VariantProps } from "class-variance-authority";

const buttonVariants = cva("cursor-pointer transition-colors", {
  variants: {
    size: {
      sm: "px-2 py-1 text-sm",
      base: "px-4 py-2",
      lg: "px-6 py-3 text-lg",
    },
    variant: {
      primary: "bg-primary text-background hover:bg-primary-500 font-bold",
      secondary: "bg-neutral-800 hover:bg-neutral-800/70",
      ghost: "hover:bg-neutral-800/50",
      outline:
        "text-primary border border-neutral-800 hover:bg-neutral-800/50 text-neutral-800",
    },
  },
  defaultVariants: {
    size: "base",
    variant: "primary",
  },
});

export const Button: React.FC<
  React.ButtonHTMLAttributes<HTMLButtonElement> &
    VariantProps<typeof buttonVariants>
> = ({ className, children, ...props }) => {
  return (
    <button className={buttonVariants({ className, ...props })} {...props}>
      {children}
    </button>
  );
};
