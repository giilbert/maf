import { cva, type VariantProps } from "class-variance-authority";

const buttonVariants = cva("cursor-pointer transition-colors", {
  variants: {
    size: {
      base: "px-4 py-2",
      lg: "px-6 py-3 text-lg",
    },
    variant: {
      primary: "bg-neutral-950 text-neutral-50 hover:bg-neutral-700 font-bold",
      secondary: "bg-neutral-100 hover:bg-neutral-200",
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
