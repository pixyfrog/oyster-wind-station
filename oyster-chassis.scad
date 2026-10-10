// oyster-chassis.scad
// Sliding chassis for the Oyster island wind-station emitter tube.
// Holds the 18650 cell, the Pico, the RFM95W and the antenna feed in line
// inside a 30.5 mm PVC bore. Parameterised: every number is named and the
// physical rules are enforced by assertions at the bottom of this block.
//
// Render one station at a time to stay inside 4 GB of RAM:
//   openscad -D part=0 -o /tmp/stack.stl     oyster-chassis.scad  // full stack
//   openscad -D part=1 -o /tmp/cradle.stl    oyster-chassis.scad  // cell cradle
//   openscad -D part=2 -o /tmp/section_a.stl oyster-chassis.scad  // z 0   -> 89
//   openscad -D part=3 -o /tmp/section_b.stl oyster-chassis.scad  // z 89  -> 357

$fn = 96; // modest tessellation for a 4 GB machine

// Numeric part selector: integers only, since -D part="name" is stripped by the
// shell and silently falls through to the default branch.
//   0 = full stack   1 = cell cradle   2 = section_a (z 0->89)   3 = section_b (z 89->357)
part = 0;

// ---------------------------------------------------------------------------
// Parameter block
// ---------------------------------------------------------------------------

// --- PVC tube (measured with a caliper) ---
pvc_bore = 30.5;   // measured: PVC inner diameter
pvc_od   = 34.2;   // measured: PVC outer diameter
pvc_wall = 1.85;   // measured: PVC wall thickness

// --- Printed chassis ---
chassis_od   = 29.7;                          // derived: pvc_bore - 0.8 mm diametral clearance
chassis_wall = 1.8;                           // design: printed wall
chassis_bore = chassis_od - 2 * chassis_wall; // derived: ~26.1 mm internal bore

// --- Printer ---
printer_z = 300;   // measured: Voron usable Z

// --- Components (measured) ---
cell_d      = 18.4;  // measured: protected 18650 diameter
cell_l      = 69.0;  // measured: protected 18650 length (protected ~5 mm longer than bare)
cell_hold_w = 21.0;  // measured: cell-in-cradle width
cell_hold_h = 17.0;  // measured: cell-in-cradle depth
pico_w      = 21.0;  // measured: Pico width  (CONFIRM with calipers before printing)
pico_l      = 51.0;  // measured: Pico length (CONFIRM with calipers before printing)
pico_t      = 4.0;   // measured: Pico thickness
pico_bay_t  = 6.0;   // design: Pico + solder joints, no Dupont header stack
radio_w     = 16.0;  // measured: RFM95W width
radio_l     = 16.0;  // measured: RFM95W length

// --- Cell cradle (design) ---
cradle_od   = 21.6;                 // design: snap-in thin C-sleeve outer diameter
cradle_wall = 1.6;                  // design: thin wall so the snap can flex
cradle_grip = cell_d + 0.4;         // derived: 18.8 mm grip bore
cradle_slot = 0.70 * cell_d;        // derived: slot width ~70 % of cell diameter

// --- RF / antenna (physics, do not change) ---
freq_mhz      = 868;              // frozen: LoRa centre frequency
lambda        = 300 / freq_mhz * 1000; // derived: ~345.6 mm at 868 MHz
lambda_quarter = lambda / 4;     // derived: ~86.4 mm
lambda_tenth   = lambda / 10;    // derived: ~34.6 mm
radiator_len  = 86.0;            // design: straight lambda/4 wire on axis
clearance_min = 50.0;            // design: minimum radiator clearance (>= lambda/10 ideal 80-100)

// --- Station layout, bottom (z=0) to top (measured/derived stack-up) ---
cap_h        = 20.0;                     // service cap + desiccant space
cell_z0      = cap_h;                    // 20
cell_z1      = cell_z0 + cell_l;         // 89
pico_bay_l   = 56.0;                     // Pico bay, wires leaving axially
pico_z0      = cell_z1;                  // 89
pico_z1      = pico_z0 + pico_bay_l;     // 145
sleeve_l     = 90.0;                     // counterpoise sleeve carrier
sleeve_z0    = pico_z1;                  // 145
sleeve_z1    = sleeve_z0 + sleeve_l;     // 235
radio_z0     = sleeve_z1;                // 235
radio_z1     = radio_z0 + radio_l;       // 251
radiator_z0  = radio_z1;                 // 251
radiator_z1  = radiator_z0 + radiator_len; // 337
headroom     = 20.0;                     // headroom to the sealed top
total_l      = radiator_z1 + headroom;   // 357

split_z      = cell_z1;                  // 89: joint in the plain region above the cell
section_a_l  = split_z;                  // 0   -> 89
section_b_l  = total_l - split_z;        // 89  -> 357

// --- Counterpoise sleeve carrier (design) ---
sleeve_boss_od = 20.0;   // design: slides inside a 22 mm copper-pipe offcut (ID ~20.2)

// --- Derived clearances: radiator start to the hottest conductors below it ---
clearance_cell = radiator_z0 - cell_z1;  // derived: 251 - 89  = 162
clearance_pico = radiator_z0 - pico_z1;  // derived: 251 - 145 = 106

// ---------------------------------------------------------------------------
// Assertions: encode the physical rules. Do not loosen to make a render pass.
// ---------------------------------------------------------------------------
assert(chassis_od < pvc_bore,
       "chassis OD must be under the PVC bore");
assert(chassis_bore > cell_d,
       "chassis bore must clear the cell diameter");
assert(chassis_bore > cell_hold_w,
       "chassis bore must clear the cell-in-cradle width");
assert(sqrt(pico_bay_t * pico_bay_t + pico_w * pico_w) < chassis_bore,
       "Pico cross-section diagonal must clear the chassis bore");
assert(cell_z1 <= pico_z0,
       "cell and Pico must not overlap axially (they sit in series)");
assert(clearance_cell >= clearance_min,
       "radiator clearance to the cell must be >= clearance_min");
assert(clearance_pico >= clearance_min,
       "radiator clearance to the Pico must be >= clearance_min");
assert(section_a_l <= printer_z && section_b_l <= printer_z,
       "each printed segment must fit under the printer height");

// ---------------------------------------------------------------------------
// ECHO report
// ---------------------------------------------------------------------------
echo(str("chassis OD      = ", chassis_od, " mm  (PVC bore ", pvc_bore, " mm)"));
echo(str("chassis bore    = ", chassis_bore, " mm"));
echo(str("total length    = ", total_l, " mm  (section_a ", section_a_l,
         " + section_b ", section_b_l, ")"));
echo(str("segment split   = z ", split_z, " mm"));
echo(str("lambda/4        = ", lambda_quarter, " mm  (radiator ", radiator_len, " mm)"));
echo(str("lambda/10       = ", lambda_tenth, " mm  (min clearance ", clearance_min, " mm)"));
echo(str("radiator->cell  = ", clearance_cell, " mm"));
echo(str("radiator->Pico  = ", clearance_pico, " mm"));
