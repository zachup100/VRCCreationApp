export const LIMIT_CHOICES = [0, 1, 2, 4, 8, 16, 24, 32];

export function limitLabel(value: number): string {
	return value === 0 ? "Unlimited" : `${value} MB/s`;
}

export function SpeedLimit({ value, onChange, disabled }: { value: number; onChange: (value: number) => void; disabled?: boolean }) {
	const found = LIMIT_CHOICES.indexOf(value);
	const index = found === -1 ? 0 : found;
	return (
		<div className="speed-limit">
			<strong className="speed-value" aria-hidden="true">
				{limitLabel(LIMIT_CHOICES[index])}
			</strong>
			<input
				type="range"
				min={0}
				max={LIMIT_CHOICES.length - 1}
				step={1}
				value={index}
				disabled={disabled}
				aria-label="Download speed limit"
				aria-valuetext={limitLabel(LIMIT_CHOICES[index])}
				onChange={(event) => onChange(LIMIT_CHOICES[Number(event.target.value)])}
			/>
			<div className="speed-ticks" aria-hidden="true">
				{LIMIT_CHOICES.map((choice) => (
					<span key={choice}>{choice === 0 ? "Unlimited" : choice}</span>
				))}
			</div>
		</div>
	);
}
