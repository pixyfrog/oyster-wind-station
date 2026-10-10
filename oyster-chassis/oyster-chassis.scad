// oyster-chassis.scad
// Skeletal, modular chassis for the Oyster island wind-station emitter tube.
//
// Five independent parts stack bottom-to-top inside the PVC bore (the PVC tube
// is NOT modelled). Each part is a small skeleton: two transverse ribs (hoops)
// joined by partial outer-shell arcs, plus one station-specific feature. The
// arcs give real FDM surface instead of thin rods. The parts are printed
// separately and glued into a chain; only the counterpoise carrier has a full
// shell, which receives the brass/copper tube.
//
//   openscad -D part=0 -o /tmp/all.stl          oyster-chassis.scad  // assembled
//   openscad -D part=1 -o /tmp/battery.stl      oyster-chassis.scad  // battery cradle
//   openscad -D part=2 -o /tmp/pico.stl         oyster-chassis.scad  // Pico bay
//   openscad -D part=3 -o /tmp/counterpoise.stl oyster-chassis.scad  // counterpoise
//   openscad -D part=4 -o /tmp/radio.stl        oyster-chassis.scad  // radio mount
//   openscad -D part=5 -o /tmp/antenna.stl      oyster-chassis.scad  // antenna guide

$fn = 96; // modest tessellation for a 4 GB machine

// Numeric part selector: integers only (-D part="name" is stripped by the shell
// and falls through to the default branch).
//   0 = assembled   1 = battery cradle   2 = Pico bay
//   3 = counterpoise carrier   4 = radio mount   5 = antenna guide
part = 0;

// ---------------------------------------------------------------------------
// Parameter block
// ---------------------------------------------------------------------------

// --- PVC tube (measured with a caliper) ---
pvc_bore = 30.5;   // measured: PVC inner diameter
pvc_od   = 34.2;   // measured: PVC outer diameter
pvc_wall = 1.85;   // measured: PVC wall thickness

// --- Printed chassis ---
chassis_od   = 29.7;   // derived: pvc_bore - 0.8 mm diametral clearance (envelope)
chassis_bore = 26.1;   // derived: reference internal envelope

// --- Printer ---
printer_z = 300;   // measured: Voron usable Z

// --- Components (measured) ---
cell_d      = 18.4;  // measured: protected 18650 diameter
cell_l      = 69.0;  // measured: protected 18650 length
pico_w      = 21.0;  // measured: Pico width  (CONFIRM with calipers before printing)
pico_l      = 51.0;  // measured: Pico length (CONFIRM with calipers before printing)
pico_bay_t  = 6.0;   // design: Pico + solder joints, no Dupont header stack
radio_w     = 16.0;  // measured: RFM95W width
radio_l     = 16.0;  // measured: RFM95W length

// --- Skeleton frame (design): two ribs (hoops) + partial shell arcs ---
collar_od   = 29.0;  // design: rib outer diameter (slides in the 30.5 bore)
collar_wall = 2.0;   // design: shell / rib wall
collar_h    = 4.0;   // design: transverse rib axial thickness
arc_a_sweep = 90.0;  // design: main partial-shell arc
arc_a_center = 90.0; // design: main arc centre (top)
arc_b_sweep = 45.0;  // design: opposite partial-shell arc
arc_b_center = 270.0;// design: opposite arc centre (bottom)
fin_t       = 2.0;   // design: vertical radial fin thickness

// --- Interlocking crenel keys at the hoop ends (design, FDM-friendly) ---
// Each part's top mates any part's bottom: 3 teeth alternate with 3 notches.
key_tooth_deg   = 45.0;   // design: tooth arc width
key_clear_deg   = 3.0;    // design: extra half-width per notch side (clearance)
key_notch_deg   = key_tooth_deg + 2 * key_clear_deg; // derived: 51 deg
key_h           = 2.5;    // design: tooth engagement depth
key_axial_clear = 0.5;    // design: axial clearance (glue room, no bottoming out)
key_notch_depth = key_h + key_axial_clear; // derived: 3.0 mm
top_teeth       = [90, 210, 330];  // top hoop teeth (one marks the 90 deg arc)
bottom_teeth    = [30, 150, 270];  // bottom hoop teeth (complementary)

// --- On-axis wire guide (design) ---
wire_guide_od   = 6.0; // design: guide tube OD
wire_guide_bore = 1.6; // design: guide bore for the wire

// --- Battery cradle feature (design) ---
// The part receives a rectangular (purchased) cradle, stuck on a flat table.
battery_table_w = 18.0;  // design: flat table width (x)
battery_table_y = -8.5;  // design: table inner face (cradle bonds here)
battery_table_t = 2.5;   // design: table thickness

// --- Pico bay feature (design) ---
pico_deck_t = 2.0;   // design: deck thickness
pico_rail_t = 1.5;   // design: side-rail thickness
pico_usb_gap = 2.0;  // design: clear gap at the bottom for the USB-C port

// --- Counterpoise carrier feature (design) ---
cp_boss_od = 20.0;   // design: boss OD, slides inside a 22 mm copper-pipe offcut (ID ~20.2)
cp_boss_id = 16.0;   // design: hollow boss bore

// --- Radio mount feature (design) ---
radio_slot_t = 4.0;  // design: board slot depth (board + components)

// --- RF / antenna (physics, do not change) ---
freq_mhz       = 868;              // frozen: LoRa centre frequency
lambda         = 300 / freq_mhz * 1000; // derived: ~345.6 mm at 868 MHz
lambda_quarter = lambda / 4;       // derived: ~86.4 mm
lambda_tenth   = lambda / 10;      // derived: ~34.6 mm
radiator_len   = 86.0;             // design: straight lambda/4 wire on axis
clearance_min  = 50.0;             // design: minimum radiator clearance

// --- Station layout, bottom (z=0) to top (measured/derived stack-up) ---
cap_h        = 20.0;                     // service cap + desiccant space
cell_z0      = cap_h;                    // 20
pico_z0      = cell_z0 + cell_l;         // 89
cp_z0        = pico_z0 + 56.0;           // 145
radio_z0     = cp_z0 + 90.0;             // 235
radiator_z0  = radio_z0 + radio_l;       // 251
radiator_z1  = radiator_z0 + radiator_len; // 337
headroom     = 20.0;                     // headroom to the sealed top
total_l      = radiator_z1 + headroom;   // 357

battery_part_l = cell_l;                 // 69  (station 1)
pico_part_l    = 56.0;                   // 56  (station 2)
cp_part_l      = 90.0;                   // 90  (station 3)
radio_part_l   = radio_l;                // 16  (station 4)
antenna_part_l = radiator_len;           // 86  (station 5)

// --- Derived clearances: radiator start to the hottest conductors below it ---
clearance_cell = radiator_z0 - (cell_z0 + cell_l); // 251 - 89  = 162
clearance_pico = radiator_z0 - cp_z0;              // 251 - 145 = 106

// ---------------------------------------------------------------------------
// Assertions: encode the physical rules. Do not loosen to make a render pass.
// ---------------------------------------------------------------------------
assert(collar_od < pvc_bore,
       "skeleton rib OD must be under the PVC bore");
assert(chassis_od < pvc_bore,
       "chassis envelope must be under the PVC bore");
assert(chassis_bore > cell_d,
       "chassis bore must clear the cell diameter");
assert(sqrt(pico_bay_t * pico_bay_t + pico_w * pico_w) < chassis_bore,
       "Pico cross-section diagonal must clear the chassis bore");
assert(pico_z0 >= cell_z0 + cell_l,
       "cell and Pico must not overlap axially (they sit in series)");
assert(clearance_cell >= clearance_min,
       "radiator clearance to the cell must be >= clearance_min");
assert(clearance_pico >= clearance_min,
       "radiator clearance to the Pico must be >= clearance_min");
assert(max(battery_part_l, max(pico_part_l, max(cp_part_l,
       max(radio_part_l, antenna_part_l)))) <= printer_z,
       "each printed part must fit under the printer height");
assert(key_h < collar_h && key_notch_depth < collar_h,
       "crenel teeth and notches must fit inside the hoop height");
assert(key_notch_deg < 120,
       "three crenel notches must fit around the hoop");

// ---------------------------------------------------------------------------
// ECHO report
// ---------------------------------------------------------------------------
echo(str("skeleton OD     = ", collar_od, " mm  (PVC bore ", pvc_bore, " mm)"));
echo(str("envelope bore   = ", chassis_bore, " mm"));
echo(str("total length    = ", total_l, " mm"));
echo(str("part lengths    = battery ", battery_part_l, ", pico ", pico_part_l,
         ", counterpoise ", cp_part_l, ", radio ", radio_part_l,
         ", antenna ", antenna_part_l));
echo(str("shell arcs      = ", arc_a_sweep, " deg top / ", arc_b_sweep, " deg bottom"));
echo(str("crenel keys     = 3 teeth, ", key_tooth_deg, " deg tooth / ", key_notch_deg,
         " deg notch, ", key_h, " mm deep, ", 2 * key_clear_deg,
         " deg + ", key_axial_clear, " mm clearance"));
echo(str("lambda/4        = ", lambda_quarter, " mm  (radiator ", radiator_len, " mm)"));
echo(str("lambda/10       = ", lambda_tenth, " mm  (min clearance ", clearance_min, " mm)"));
echo(str("radiator->cell  = ", clearance_cell, " mm"));
echo(str("radiator->Pico  = ", clearance_pico, " mm"));

// ---------------------------------------------------------------------------
// Common skeleton
// ---------------------------------------------------------------------------

// One transverse rib: a thin hoop that rides the PVC bore and centres the part.
module hoop(z) {
    translate([0, 0, z])
        difference() {
            cylinder(d = collar_od, h = collar_h);
            translate([0, 0, -0.1])
                cylinder(d = collar_od - 2 * collar_wall, h = collar_h + 0.2);
        }
}

// Crenel teeth: arc-wall segments added at a given axial band, at `angles`.
module key_teeth(z0, z1, angles, sweep) {
    for (a = angles) arc_wall(a, sweep, z0, z1);
}

// Apply the crenel keys to a whole part end. The keys are cut/added to the
// union of EVERYTHING at that end (base hoop + shell arcs + features), so the
// neighbour's teeth clear the arcs as well as the hoop. key_bottom/key_top
// false leaves a free (plain) end for the outermost parts. Tooth tips are left
// flat: on the bottom end that is the first layer, so it wants full surface.
module apply_keys(L, key_bottom = true, key_top = true) {
    difference() {
        union() {
            children();
            if (key_bottom)
                key_teeth(-key_h, 0.5, bottom_teeth, key_tooth_deg);
            if (key_top)
                key_teeth(L - 0.5, L + key_h, top_teeth, key_tooth_deg);
        }
        // bottom notches receive the part below's upward teeth
        if (key_bottom)
            key_teeth(-0.2, key_notch_depth, top_teeth, key_notch_deg);
        // top notches receive the part above's downward teeth
        if (key_top)
            key_teeth(L - key_notch_depth, L + 0.2, bottom_teeth, key_notch_deg);
    }
}

// A pie-sector wedge (r = 0 -> large) used to cut a partial-shell arc. Built
// from a triangle so it works on OpenSCAD 2021.01, whose cylinder() has no
// working `angle` parameter.
module wedge(center, sweep, h) {
    rotate([0, 0, center - sweep / 2])
        linear_extrude(height = h)
            polygon([[0, 0], [100, 0],
                     [100 * cos(sweep), 100 * sin(sweep)]]);
}

// One longitudinal partial-shell arc at the outer wall radius.
module arc_wall(center, sweep, z0, z1) {
    translate([0, 0, z0])
        intersection() {
            difference() {
                cylinder(d = collar_od, h = z1 - z0);
                translate([0, 0, -0.1])
                    cylinder(d = collar_od - 2 * collar_wall, h = z1 - z0 + 0.2);
            }
            wedge(center, sweep, z1 - z0);
        }
}

// The two longitudinal arcs: 90 degrees on one side, 45 on the opposite side.
// big_center sets which side carries the 90 degree arc (90 = top, 270 = bottom).
module shell_arcs(z0, z1, big_center = arc_a_center) {
    arc_wall(big_center, arc_a_sweep, z0, z1);
    arc_wall(big_center + 180, arc_b_sweep, z0, z1);
}

// A vertical radial fin (prints as a wall) between two radii.
module fin(angle, r_in, r_out, z0, z1) {
    rotate([0, 0, angle])
        translate([(r_in + r_out) / 2, 0, (z0 + z1) / 2])
            cube([r_out - r_in, fin_t, z1 - z0], center = true);
}

// An on-axis wire guide tube held in the middle of the part.
module wire_guide(z0, z1) {
    translate([0, 0, z0])
        difference() {
            cylinder(d = wire_guide_od, h = z1 - z0);
            translate([0, 0, -0.1])
                cylinder(d = wire_guide_bore, h = z1 - z0 + 0.2);
        }
}

// ---------------------------------------------------------------------------
// Station 1: battery cradle carrier
// Two hoops + partial arcs (90 degree arc at the bottom) + a flat table on the
// 90 degree arc. A rectangular 18650 cradle is stuck to the table with
// double-sided adhesive.
// ---------------------------------------------------------------------------
module battery_cradle() {
    L  = battery_part_l;
    tw = battery_table_w;
    ty = battery_table_y;          // inner (bonding) face
    tt = battery_table_t;
    apply_keys(L, key_bottom = false, key_top = true) { // bottom end is free
        hoop(0);
        hoop(L - collar_h);
        shell_arcs(0, L, 270);         // 90 degree arc on the bottom
        // flat table on the 90 degree arc (vertical plate -> prints as a wall)
        translate([-tw / 2, ty - tt, 0])
            cube([tw, tt, L]);
    }
}

// ---------------------------------------------------------------------------
// Station 2: Pico bay
// Two hoops + partial arcs + a flat deck and two side rails; the board lies
// flat with the USB-C port facing the bottom cap. Wires leave axially.
// ---------------------------------------------------------------------------
module pico_bay() {
    L        = pico_part_l;
    deck_w   = collar_od - 2 * collar_wall + 0.4; // reaches both hoops
    rail_in  = pico_w / 2 + 0.2;                  // 10.7
    rail_out = deck_w / 2;                        // 12.7
    z0       = pico_usb_gap;
    apply_keys(L) {
        hoop(0);
        hoop(L - collar_h);
        shell_arcs(0, L, 270); // 90 degree arc below the Pico table
        // deck (also a longitudinal member)
        translate([-deck_w / 2, -(pico_bay_t / 2 + pico_deck_t), z0])
            cube([deck_w, pico_deck_t, L - z0]);
        // side rails on the deck
        for (x = [-1, 1])
            translate([x > 0 ? rail_in : -rail_out, -pico_bay_t / 2 - 1, z0])
                cube([rail_out - rail_in, pico_bay_t + 1, L - z0]);
    }
}

// ---------------------------------------------------------------------------
// Station 3: counterpoise carrier
// Two hoops + a full hollow boss the brass/copper tube slides over (the mass),
// with vertical fins to the hoops and an on-axis wire guide inside.
// ---------------------------------------------------------------------------
module counterpoise_carrier() {
    L = cp_part_l;
    apply_keys(L) {
        hoop(0);
        hoop(L - collar_h);
        // full shell boss for the brass tube
        difference() {
            cylinder(d = cp_boss_od, h = L);
            translate([0, 0, -0.1])
                cylinder(d = cp_boss_id, h = L + 0.2);
        }
        // vertical fins: boss -> hoops
        for (a = [0, 120, 240])
            fin(a, cp_boss_od / 2 - 0.5, collar_od / 2 - collar_wall + 0.2, 0, L);
        // on-axis wire guide + fins to the boss
        wire_guide(0, L);
        for (a = [0, 180])
            fin(a, wire_guide_od / 2 - 0.5, cp_boss_id / 2 + 0.5, 0, L);
    }
}

// ---------------------------------------------------------------------------
// Station 4: radio mount
// Like the Pico bay: two hoops + partial arcs (90 degree arc at the bottom) +
// a flat table with two locating rails that receive the RFM95W board. Height
// stays 16 mm per spec. A cable tie-down loop and an on-axis lead hole.
// ---------------------------------------------------------------------------
module radio_mount() {
    L        = radio_part_l;
    deck_w   = collar_od - 2 * collar_wall + 0.4; // reaches both hoops
    rail_in  = radio_w / 2 + 0.2;                 // 8.2
    rail_out = deck_w / 2;                        // 12.7
    apply_keys(L) {
        hoop(0);
        hoop(L - collar_h);
        shell_arcs(0, L, 270);        // 90 degree arc below the board table
        // table (also a longitudinal member)
        translate([-deck_w / 2, -(radio_slot_t / 2 + pico_deck_t), 0])
            cube([deck_w, pico_deck_t, L]);
        // two locating rails on the table
        for (x = [-1, 1])
            translate([x > 0 ? rail_in : -rail_out, -radio_slot_t / 2 - 1, 0])
                cube([rail_out - rail_in, radio_slot_t + 1, L]);
        // cable tie-down loop on the lower arc
        rotate([0, 0, 270])
            translate([10.75, 0, 2])
                difference() {
                    cube([6, 3.5, 5], center = true);
                    cylinder(d = 3, h = 6, center = true);
                }
    }
}

// ---------------------------------------------------------------------------
// Station 5: antenna guide
// Two hoops + partial arcs + an on-axis wire guide tube, joined by vertical
// fins to the arcs. Deliberately open: no full shell around the radiator.
// ---------------------------------------------------------------------------
module antenna_guide() {
    L = antenna_part_l;
    apply_keys(L, key_bottom = true, key_top = false) { // top end is free
        hoop(0);
        hoop(L - collar_h);
        shell_arcs(0, L);
        wire_guide(0, L);
        for (a = [arc_a_center, arc_b_center])
            fin(a, wire_guide_od / 2 - 0.5, collar_od / 2 - collar_wall + 0.2, 0, L);
    }
}

// ---------------------------------------------------------------------------
// Assembly and dispatch
// ---------------------------------------------------------------------------
module assembled() {
    translate([0, 0, cell_z0])    battery_cradle();
    translate([0, 0, pico_z0])    pico_bay();
    translate([0, 0, cp_z0])      counterpoise_carrier();
    translate([0, 0, radio_z0])   radio_mount();
    translate([0, 0, radiator_z0]) antenna_guide();
}

if (part == 0) {
    assembled();
} else if (part == 1) {
    battery_cradle();
} else if (part == 2) {
    pico_bay();
} else if (part == 3) {
    counterpoise_carrier();
} else if (part == 4) {
    radio_mount();
} else if (part == 5) {
    antenna_guide();
}
