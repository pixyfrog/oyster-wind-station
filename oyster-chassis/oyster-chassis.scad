// oyster-chassis.scad
// Skeletal, modular chassis for the Oyster island wind-station emitter tube.
//
// Five independent parts stack bottom-to-top inside the PVC bore (the PVC tube
// is NOT modelled). Each part is a small skeleton: two transverse ribs (hoops)
// joined by longitudinal members, plus one station-specific feature. The parts
// are printed separately and glued into a chain; the only station with an outer
// shell is the counterpoise carrier, which receives the brass/copper tube.
//
//   openscad -D part=0 -o /tmp/all.stl        oyster-chassis.scad  // assembled
//   openscad -D part=1 -o /tmp/battery.stl    oyster-chassis.scad  // battery cradle
//   openscad -D part=2 -o /tmp/pico.stl       oyster-chassis.scad  // Pico bay
//   openscad -D part=3 -o /tmp/counterpoise.stl oyster-chassis.scad // counterpoise
//   openscad -D part=4 -o /tmp/radio.stl      oyster-chassis.scad  // radio mount
//   openscad -D part=5 -o /tmp/antenna.stl    oyster-chassis.scad  // antenna guide

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

// --- Skeleton frame (design): two ribs (hoops) + longitudinal members ---
collar_od   = 29.0;  // design: rib outer diameter (slides in the 30.5 bore)
collar_wall = 2.0;   // design: rib radial wall
collar_h    = 4.0;   // design: rib axial thickness
rod_d       = 2.5;   // design: longitudinal rod diameter
rod_r       = 12.5;  // design: rod pitch radius (overlaps the rib inner face)
rod_angles  = [0, 120, 240]; // design: three rods at 120 degrees

// --- Battery cradle feature (design) ---
trough_od   = collar_od - 2 * collar_wall + 0.4; // derived: lower half-pipe OD
trough_id   = cell_d + 0.6;                      // derived: half-pipe bore
strap_h     = 3.0;                               // design: cell retaining strap height
strap_wall  = 1.2;                               // design: cell retaining strap wall

// --- Pico bay feature (design) ---
pico_deck_t = 2.0;   // design: deck thickness
pico_rail_t = 1.5;   // design: side-rail thickness
pico_usb_gap = 2.0;  // design: clear gap at the bottom for the USB-C port

// --- Counterpoise carrier feature (design) ---
cp_boss_od = 20.0;   // design: boss OD, slides inside a 22 mm copper-pipe offcut (ID ~20.2)
cp_boss_id = 16.0;   // design: hollow boss bore

// --- Radio mount feature (design) ---
radio_wall_y = 2.5;  // design: cradle wall half-depth

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

// ---------------------------------------------------------------------------
// ECHO report
// ---------------------------------------------------------------------------
echo(str("skeleton OD     = ", collar_od, " mm  (PVC bore ", pvc_bore, " mm)"));
echo(str("envelope bore   = ", chassis_bore, " mm"));
echo(str("total length    = ", total_l, " mm"));
echo(str("part lengths    = battery ", battery_part_l, ", pico ", pico_part_l,
         ", counterpoise ", cp_part_l, ", radio ", radio_part_l,
         ", antenna ", antenna_part_l));
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

// One longitudinal rod between two axial stations.
module rod(angle, z0, z1) {
    rotate([0, 0, angle])
        translate([rod_r, 0, (z0 + z1) / 2])
            cylinder(d = rod_d, h = z1 - z0, center = true);
}

module rods(z0, z1) {
    for (a = rod_angles) rod(a, z0, z1);
}

// ---------------------------------------------------------------------------
// Station 1: battery cradle
// Two hoops + a lower half-pipe trough the 18650 drops into + two full straps
// that retain it. The trough reaches both hoops, so it is the main member.
// ---------------------------------------------------------------------------
module battery_cradle() {
    L = battery_part_l;
    hoop(0);
    hoop(L - collar_h);
    // lower half-pipe trough
    difference() {
        cylinder(d = trough_od, h = L);
        translate([0, 0, -0.1])
            cylinder(d = trough_id, h = L + 0.2);
        // open the top half
        translate([0, trough_od / 2, L / 2])
            cube([trough_od + 4, trough_od, L + 2], center = true);
    }
    // two retaining straps over the cell
    for (z = [15, L - 18])
        translate([0, 0, z])
            difference() {
                cylinder(d = cell_d + 2 * strap_wall + 0.6, h = strap_h);
                translate([0, 0, -0.1])
                    cylinder(d = cell_d + 0.6, h = strap_h + 0.2);
            }
    rods(0, L);
}

// ---------------------------------------------------------------------------
// Station 2: Pico bay
// Two hoops + a flat deck and two side rails; the board lies flat with the
// USB-C port facing the bottom cap (open gap at the bottom). Wires leave
// axially through the open frame.
// ---------------------------------------------------------------------------
module pico_bay() {
    L        = pico_part_l;
    deck_w   = collar_od - 2 * collar_wall + 0.4; // reaches both hoops
    rail_in  = pico_w / 2 + 0.2;                  // 10.7
    rail_out = deck_w / 2;                        // 12.7
    z0       = pico_usb_gap;
    hoop(0);
    hoop(L - collar_h);
    // deck (also the main longitudinal member)
    translate([-deck_w / 2, -(pico_bay_t / 2 + pico_deck_t), z0])
        cube([deck_w, pico_deck_t, L - z0]);
    // side rails on the deck
    for (x = [-1, 1])
        translate([x > 0 ? rail_in : -rail_out, -pico_bay_t / 2 - 1, z0])
            cube([rail_out - rail_in, pico_bay_t + 1, L - z0]);
    // two top rods for rigidity
    rod(60, 0, L);
    rod(300, 0, L);
}

// ---------------------------------------------------------------------------
// Station 3: counterpoise carrier
// Two hoops + a hollow boss on the axis that the brass/copper tube slides over
// (the mass), joined by three radial longitudinal members.
// ---------------------------------------------------------------------------
module counterpoise_carrier() {
    L     = cp_part_l;
    r_in  = cp_boss_od / 2 - 0.5;                 // 9.5, penetrates the boss
    r_out = collar_od / 2 - collar_wall + 0.2;    // 12.7, penetrates the hoop
    hoop(0);
    hoop(L - collar_h);
    difference() {
        cylinder(d = cp_boss_od, h = L);
        translate([0, 0, -0.1])
            cylinder(d = cp_boss_id, h = L + 0.2);
    }
    for (a = rod_angles)
        rotate([0, 0, a])
            translate([(r_in + r_out) / 2, 0, L / 2])
                cube([r_out - r_in, 2.5, L], center = true);
}

// ---------------------------------------------------------------------------
// Station 4: radio mount
// Two hoops + two cradle walls that hold the RFM95W vertically (ANT pad up,
// GND pad down) on the axis, plus a cable tie-down loop.
// ---------------------------------------------------------------------------
module radio_mount() {
    L        = radio_part_l;
    wall_in  = radio_w / 2 + 0.2;                 // 8.2
    wall_out = collar_od / 2 - collar_wall + 0.2; // 12.7
    hoop(0);
    hoop(L - collar_h);
    // cradle walls (reach both hoops)
    for (x = [-1, 1])
        translate([x > 0 ? wall_in : -wall_out, -radio_wall_y, 0])
            cube([wall_out - wall_in, 2 * radio_wall_y, L]);
    // floor with a GND lead hole on axis
    translate([-wall_out, -radio_wall_y, 0])
        cube([2 * wall_out, 2 * radio_wall_y, 2]);
    translate([0, 0, -0.1]) cylinder(d = 6, h = 3);
    // cable tie-down loop on the lower hoop
    rotate([0, 0, 270])
        translate([10.75, 0, 2])
            difference() {
                cube([6, 3.5, 5], center = true);
                cylinder(d = 3, h = 6, center = true);
            }
}

// ---------------------------------------------------------------------------
// Station 5: antenna guide
// Two hoops + three longitudinal rods + three hub collars on the axis that
// hold the straight lambda/4 wire. Deliberately skeletal: thin guides only.
// ---------------------------------------------------------------------------
module antenna_guide() {
    L      = antenna_part_l;
    levels = [0, L / 2, L - collar_h];
    hoop(0);
    hoop(L - collar_h);
    rods(0, L);
    for (z = levels) {
        translate([0, 0, z]) difference() {
            cylinder(d = 4, h = 2);
            translate([0, 0, -0.1]) cylinder(d = 1.6, h = 2.2);
        }
        for (a = rod_angles)
            rotate([0, 0, a])
                translate([0, 0, z + 1])
                    rotate([0, 90, 0])
                        cylinder(d = 2, h = rod_r);
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
