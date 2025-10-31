import { cn } from "@/lib/cn";

export const Logo: React.FC<{
  size?: number;
  className?: string;
  hasText?: boolean;
}> = ({ size = 32, className, hasText }) => {
  return (
    <div className="flex gap-4 items-center">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={size}
        height={size}
        viewBox="0 0 128 128"
        fill="none"
        className={cn("text-foreground", className)}
      >
        <path
          d="M71.0711 14.5025C74.9763 18.4078 74.9763 24.7394 71.0711 28.6447L21.5736 78.1421L7.43146 64L56.9289 14.5025C60.8342 10.5973 67.1658 10.5973 71.0711 14.5025V14.5025Z"
          fill="currentColor"
        />
        <path
          d="M56.9289 113.497C53.0237 109.592 53.0237 103.261 56.9289 99.3554L106.426 49.8579L120.569 64L71.0711 113.497C67.1658 117.403 60.8342 117.403 56.9289 113.497V113.497Z"
          fill="currentColor"
        />
        <path
          d="M35.7157 92.2843C31.8105 88.379 31.8105 82.0474 35.7157 78.1421L71.0711 42.7868L85.2132 56.9289L49.8579 92.2843C45.9526 96.1895 39.621 96.1895 35.7157 92.2843V92.2843Z"
          fill="currentColor"
        />
      </svg>

      {hasText && <p className="text-2xl font-bold">Cobble</p>}
    </div>
  );
};
