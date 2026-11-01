import { Plus, Trash01 } from "@untitledui/icons";
import { Button } from "@/components/base/buttons/button";
import { ButtonGroup, ButtonGroupItem } from "@/components/base/button-group/button-group";
import { InputNumber } from "@/components/base/input/input-number";
import {
  IDENTITY_POINTS,
  curvePath,
  normalizePoints,
  pointsOf,
  type CurvePoint,
  type CurveType,
  type PressureCurve,
} from "@/features/pen/curve";
import { useMappingSnapshot, useMappingStore } from "@/features/mapping/MappingProvider";
import { Section } from "@/ui/Page";

const TYPES: { id: CurveType; label: string }[] = [
  { id: "Linear", label: "Linear" },
  { id: "Exponential", label: "Exponential" },
  { id: "Custom", label: "Custom" },
];

const PLOT = 200;

/** The shape of the curve, drawn from the same arithmetic as the driver. */
function CurvePlot({ curve }: { curve: PressureCurve }) {
  const points = curve.curve_type === "Custom" ? pointsOf(curve) : [];
  return (
    <svg
      role="img"
      aria-label={`Pressure curve, ${curve.curve_type}: output pressure against raw pressure`}
      viewBox={`-24 -8 ${String(PLOT + 40)} ${String(PLOT + 36)}`}
      className="h-56 w-full max-w-72 text-fg-quaternary"
    >
      <rect
        x="0"
        y="0"
        width={PLOT}
        height={PLOT}
        className="fill-secondary stroke-secondary"
        strokeWidth="1"
      />
      <line x1="0" y1={PLOT} x2={PLOT} y2="0" className="stroke-tertiary" strokeDasharray="4 4" />
      <polyline
        fill="none"
        points={curvePath(curve, PLOT)}
        className="stroke-fg-brand-primary"
        strokeWidth="2.5"
        strokeLinejoin="round"
      />
      {points.map(([x, y], index) => (
        <circle
          key={index}
          cx={x * PLOT}
          cy={(1 - y) * PLOT}
          r="4.5"
          className="fill-bg-brand-solid stroke-white"
          strokeWidth="1.5"
        />
      ))}
      <text x={PLOT / 2} y={PLOT + 22} textAnchor="middle" className="fill-current text-xs">
        Raw
      </text>
      <text
        x="-14"
        y={PLOT / 2}
        textAnchor="middle"
        transform={`rotate(-90 -14 ${String(PLOT / 2)})`}
        className="fill-current text-xs"
      >
        Curve
      </text>
    </svg>
  );
}

/** Chooses how pen pressure is turned into the pressure sent to programs. */
export function PressureCurveEditor() {
  const { config } = useMappingSnapshot();
  const mapping = useMappingStore();
  if (config === null) {
    return null;
  }
  const curve = config.pressure_curve;
  const points = pointsOf(curve);

  const setCurve = (change: Partial<PressureCurve>): void => {
    mapping.update((c) => ({ ...c, pressure_curve: { ...c.pressure_curve, ...change } }));
  };
  const setPoints = (next: CurvePoint[]): void => {
    setCurve({ points: normalizePoints(next) });
  };

  return (
    <Section
      title="Pressure curve"
      description="How hard you press, and the pressure that programs receive."
    >
      <ButtonGroup
        aria-label="Curve type"
        selectedKeys={[curve.curve_type]}
        disallowEmptySelection
        onSelectionChange={(keys) => {
          const [first] = keys;
          if (first === undefined) {
            return;
          }
          const type = first as CurveType;
          setCurve({
            curve_type: type,
            ...(type === "Custom" && curve.points.length < 2
              ? { points: IDENTITY_POINTS.map(([x, y]): [number, number] => [x, y]) }
              : {}),
          });
        }}
      >
        {TYPES.map((type) => (
          <ButtonGroupItem key={type.id} id={type.id}>
            {type.label}
          </ButtonGroupItem>
        ))}
      </ButtonGroup>

      <div className="flex flex-wrap items-start gap-8">
        <CurvePlot curve={curve} />

        <div className="flex min-w-64 flex-1 flex-col gap-4">
          {curve.curve_type === "Exponential" && (
            <div className="w-40">
              <InputNumber
                label="Exponent"
                hint="Above 1 needs more pressure for the same output."
                minValue={0.1}
                maxValue={10}
                step={0.1}
                formatOptions={{ maximumFractionDigits: 2, useGrouping: false }}
                value={curve.exponent ?? 1}
                onChange={(exponent) => {
                  if (Number.isFinite(exponent)) {
                    setCurve({ exponent });
                  }
                }}
              />
            </div>
          )}

          {curve.curve_type === "Custom" && (
            <div className="flex flex-col gap-3">
              <p className="text-sm text-tertiary">
                Control points, from no pressure to full pressure. The first is at 0 and the last at
                1.
              </p>
              <div aria-hidden="true" className="flex gap-2 text-sm font-medium text-secondary">
                <span className="w-28">Raw</span>
                <span className="w-28">Curve</span>
              </div>
              {points.map(([x, y], index) => {
                const isEnd = index === 0 || index === points.length - 1;
                return (
                  <div key={index} className="flex items-center gap-2">
                    <div className="w-28">
                      <InputNumber
                        aria-label={`Point ${String(index + 1)}: raw`}
                        minValue={0}
                        maxValue={1}
                        step={0.05}
                        isDisabled={isEnd}
                        formatOptions={{ maximumFractionDigits: 3 }}
                        value={x}
                        onChange={(value) => {
                          if (Number.isFinite(value)) {
                            setPoints(
                              points.map((p, i): CurvePoint => (i === index ? [value, p[1]] : p)),
                            );
                          }
                        }}
                      />
                    </div>
                    <div className="w-28">
                      <InputNumber
                        aria-label={`Point ${String(index + 1)}: curve`}
                        minValue={0}
                        maxValue={1}
                        step={0.05}
                        formatOptions={{ maximumFractionDigits: 3 }}
                        value={y}
                        onChange={(value) => {
                          if (Number.isFinite(value)) {
                            setPoints(
                              points.map((p, i): CurvePoint => (i === index ? [p[0], value] : p)),
                            );
                          }
                        }}
                      />
                    </div>
                    <Button
                      color="tertiary-destructive"
                      size="md"
                      aria-label={`Remove point ${String(index + 1)}`}
                      iconLeading={Trash01}
                      isDisabled={points.length <= 2 || isEnd}
                      onPress={() => {
                        setPoints(points.filter((_, i) => i !== index));
                      }}
                    />
                  </div>
                );
              })}
              <div>
                <Button
                  color="secondary"
                  size="sm"
                  iconLeading={Plus}
                  onPress={() => {
                    setPoints([...points, [0.5, 0.5]]);
                  }}
                >
                  Add a point
                </Button>
              </div>
            </div>
          )}
        </div>
      </div>
    </Section>
  );
}
