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
cradle_bore = cradle_od - 2*cradle_wall; // derived: 18.4 mm resting grip bore
                                    //   (the C-slot lets the sleeve flex past the
                                    //    18.4 cell; cradle_grip is the flexed entry)
cradle_grip  = cell_d + 0.4;        // derived: 18.8 mm flexed grip / snap entry
cradle_slot  = 0.70 * cell_d;       // derived: slot width ~70 % of cell diameter
stop_ring_h  = 1.5;                 // design: bottom stop-ring shoulder, cell rests here
stop_hole_d  = cell_d - 6.0;        // derived: 12.4 mm stop-ring hole (3 mm shoulder)

// --- Pico bay (design) ---
pico_slot_w  = pico_w + 0.4;        // derived: 21.4 mm slot width
pico_slot_t  = pico_bay_t + 0.2;    // derived: 6.2 mm slot depth (no Dupont stack)
pico_rail    = 2.0;                 // design: side-rail thickness
pico_floor_t = 2.0;                 // design: floor thickness
pico_usb_gap = 4.0;                 // design: clear gap at the bottom for the USB-C port

// --- RF / antenna (physics, do not change) ---
freq_mhz      = 868;              // frozen: LoRa centre frequency
lambda        = 300 / freq_mhz * 1000; // derived: ~345.6 mm at 868 MHz
lambda_quarter = lambda / 4;     // derived: ~86.4 mm
lambda_tenth   = lambda / 10;    // derived: ~34.6 mm
radiator_len  = 86.0;            // design: straight lambda/4 wire on axis
clearance_min = 50.0;            // design: minimum radiator clearance (>= lambda/10 ideal 80-100)

// --- Joining shell (design) ---
window_w     = 10.0;       // design: axial window width
band_z       = [22.0, 205.0]; // design: foam/O-ring band positions on the OD
band_w       = 3.0;        // design: band groove width
band_depth   = 1.0;        // design: band groove depth
grip_d       = 8.0;        // design: bottom finger-grip scallop diameter
rib_th       = 2.5;        // design: rib thickness (axial)
rib_w        = 2.0;        // design: rib width (radial/tangential)
cap_l        = 18.0;       // design: removable service-cap depth

// --- Segment join at z = split_z (design: press-fit lap, never bonded) ---
joint_overlap = 6.0;       // design: axial lap length
spigot_od     = 28.0;      // design: male spigot OD (section_b)
socket_bore   = 27.7;      // design: female socket bore (section_a)

// --- Antenna guide (design, skeletal: thin guides only) ---
guide_r      = 12.0;   // design: cage radius (inside the 26.1 bore with clearance)
guide_strut  = 1.6;    // design: strut / spoke thickness
guide_hub_d  = 4.0;    // design: hub collar diameter
guide_hub_h  = 2.0;    // design: hub collar thickness
guide_bore   = 1.6;    // design: wire bore through each hub

// --- RFM95W mount (design) ---
radio_slot_w = radio_w + 0.4;  // derived: 16.4 mm board slot width
radio_slot_t = 2.0;            // design: board slot thickness (board ~1.6 + clearance)
radio_wall   = 1.6;            // design: mount wall thickness

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

shell_top    = radio_z1;                 // derived: solid shell ends at the feed (251)

// --- Counterpoise sleeve carrier (design) ---
sleeve_boss_od = 20.0;   // design: slides inside a 22 mm copper-pipe offcut (ID ~20.2)
sleeve_boss_bore = 16.0; // design: hollow boss to save filament
sleeve_ridge_d = 20.2;   // design: shallow retention ring for the copper sleeve
sleeve_ridge_h = 1.6;    // design: retention ring height

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
assert(cradle_od < chassis_bore,
       "cell cradle outer must clear the chassis bore");
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
assert(socket_bore > chassis_bore && spigot_od > socket_bore && spigot_od < chassis_od,
       "segment lap must be an interference spigot inside the socket");

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

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

// Snap-in thin C-sleeve that holds the 18650. The thin 1.6 mm wall is what lets
// the C flex; a thick ring could not. A bottom stop ring catches the cell so it
// cannot slide down. Standalone the module is built with the cell seat at
// stop_ring_h and spans z 0 -> cell_l; the full stack shifts it down by
// stop_ring_h so the seat lands exactly at cell_z0.
module cell_cradle() {
    difference() {
        union() {
            // sleeve shell
            difference() {
                cylinder(d = cradle_od, h = cell_l);
                translate([0, 0, -0.1])
                    cylinder(d = cradle_bore, h = cell_l + 0.2);
                // C-slot, opening toward +Y (kept clear of the wall ribs)
                rotate([0, 0, 90])
                    translate([cradle_od / 2, 0, cell_l / 2])
                        cube([cradle_od, cradle_slot, cell_l + 2], center = true);
            }
            // bottom stop ring (annular shoulder the cell rests on)
            difference() {
                cylinder(d = cradle_od, h = stop_ring_h);
                translate([0, 0, -0.1])
                    cylinder(d = stop_hole_d, h = stop_ring_h + 0.2);
            }
        }
    }
}

// Pico bay: an open tray with two side rails and a floor, USB-C facing the
// bottom cap (open at local z 0 -> pico_usb_gap). Wires leave axially through
// reliefs near the top. Local z origin = pico_z0.
module pico_bay() {
    sw   = pico_slot_w;
    st   = pico_slot_t;
    rail = pico_rail;
    ft   = pico_floor_t;
    z0   = pico_usb_gap;
    z1   = z0 + pico_l;
    difference() {
        union() {
            // floor under the board, centred on the axis
            translate([-(sw + 2 * rail) / 2, -st / 2 - ft / 2, z0])
                cube([sw + 2 * rail, ft, z1 - z0]);
            // two side rails, centred at x = +/- (sw/2 + rail/2)
            for (x = [-1, 1])
                translate([x * (sw / 2 + rail / 2) - rail / 2, -st / 2, z0])
                    cube([rail, st, z1 - z0]);
        }
        // axial wire reliefs near the top of each rail
        for (x = [-1, 1])
            translate([x * (sw / 2 + rail / 2), 0, z1 - 6])
                rotate([90, 0, 0])
                    cylinder(d = 3.5, h = st + 2 * rail + 2, center = true);
    }
}

// Counterpoise carrier: a hollow boss the 22 mm copper-pipe offcut slides over,
// giving ~86-90 mm of sleeve on the ground side directly below the feed. Two
// shallow retention rings hold the sleeve. Local z origin = sleeve_z0.
module sleeve_boss() {
    difference() {
        union() {
            cylinder(d = sleeve_boss_od, h = sleeve_l);
            for (z = [12, sleeve_l - 12])
                translate([0, 0, z - sleeve_ridge_h / 2])
                    cylinder(d = sleeve_ridge_d, h = sleeve_ridge_h);
        }
        translate([0, 0, -0.1])
            cylinder(d = sleeve_boss_bore, h = sleeve_l + 0.2);
    }
}

// RFM95W mount: a U-cradle holding the radio board vertically so the ANT pad
// faces up (toward the radiator) and the GND pad faces down (ground side). The
// board sits centred on the axis so the lambda/4 wire can run straight up.
// A tie-down loop below takes the cable-bundle load off the antenna pad.
// Local z origin = radio_z0.
module radio_mount() {
    bw   = radio_slot_w;
    bt   = radio_slot_t;
    bz   = radio_l;
    wall = radio_wall;
    difference() {
        union() {
            // floor under the board
            translate([-(bw / 2 + wall), -bt / 2 - wall, 0])
                cube([bw + 2 * wall, bt + 2 * wall, 2]);
            // two side walls gripping the board edges
            for (x = [-1, 1])
                translate([x * (bw / 2 + wall / 2) - wall / 2, -bt / 2 - wall, 0])
                    cube([wall, bt + 2 * wall, bz]);
        }
        // GND feed hole in the floor (on axis)
        translate([0, 0, -0.1]) cylinder(d = 6, h = 3);
        // ANT lead hole at the top (on axis)
        translate([0, 0, bz - 2]) cylinder(d = 6, h = 2.2);
    }
    // cable-bundle tie-down: a block with a 3 mm bore for a zip tie, let into
    // the floor so it prints as one body
    translate([0, -(bt / 2 + wall + 2.0), 4]) {
        difference() {
            cube([12, 6, 7], center = true);
            rotate([90, 0, 0]) cylinder(d = 3, h = 7, center = true);
        }
    }
}

// Skeletal antenna guide: three axial struts on a 24 mm cage with three hub
// collars that hold the straight lambda/4 wire on the tube axis. No solid wall
// around the radiator, so the printed plastic loads the wire as little as
// possible. Local z origin = radiator_z0.
module antenna_guide() {
    levels = [0, radiator_len / 2, radiator_len - guide_hub_h];
    // three axial struts
    for (a = [0, 120, 240])
        rotate([0, 0, a])
            translate([guide_r, 0, radiator_len / 2])
                cylinder(d = guide_strut, h = radiator_len, center = true);
    // hub collars with three radial spokes each
    for (z = levels) {
        translate([0, 0, z])
            difference() {
                cylinder(d = guide_hub_d, h = guide_hub_h);
                translate([0, 0, -0.1])
                    cylinder(d = guide_bore, h = guide_hub_h + 0.2);
            }
        for (a = [0, 120, 240])
            rotate([0, 0, a])
                translate([0, 0, z + guide_hub_h / 2])
                    rotate([0, 90, 0])
                        cylinder(d = guide_strut, h = guide_r);
    }
}

// ---------------------------------------------------------------------------
// Joining shell, ribs and service cap
// ---------------------------------------------------------------------------

// A foam / O-ring band groove: an annulus cutter that shaves the outer skin.
module band_groove() {
    difference() {
        cylinder(d = chassis_od + 2, h = band_w, center = true);
        cylinder(d = chassis_od - 2 * band_depth, h = band_w + 0.2, center = true);
    }
}

// A radial rib from r_in to r_out at an absolute height z and angle.
module rib(angle, z, r_in, r_out) {
    rotate([0, 0, angle])
        translate([(r_in + r_out) / 2, 0, z])
            cube([r_out - r_in, rib_w, rib_th], center = true);
}

// Hollow outer shell spanning absolute z0 -> z1, with axial windows between the
// rib angles, band grooves and the bottom finger grips. Solid only below the
// feed; the antenna section above is left skeletal.
module shell(z0, z1) {
    translate([0, 0, z0])
        difference() {
            cylinder(d = chassis_od, h = z1 - z0);
            translate([0, 0, -0.1])
                cylinder(d = chassis_bore, h = z1 - z0 + 0.2);
            // windows over the cell and Pico (angles clear of ribs and C-slot)
            for (a = [30, 210, 330])
                for (zc = [[cell_z0 + 8, cell_z1 - 6], [pico_z0 + 8, pico_z0 + 48]])
                    if (zc[0] < z1 && zc[1] > z0)
                        rotate([0, 0, a])
                            translate([chassis_od / 2 - 0.5, 0, (zc[0] + zc[1]) / 2 - z0])
                                cube([3, window_w, zc[1] - zc[0]], center = true);
            // foam / O-ring band grooves
            for (z = band_z)
                if (z > z0 + band_w && z < z1 - band_w)
                    translate([0, 0, z - z0]) band_groove();
            // bottom finger grips
            if (z0 < cap_h)
                for (a = [0, 120, 240])
                    rotate([0, 0, a])
                        translate([chassis_od / 2, 0, 10])
                            cylinder(d = grip_d, h = 14, center = true);
        }
}

// Removable push-in service cap: a shallow cup holding the desiccant, pulled
// with a hook through the cross-hole. Deliberately not bonded to the shell.
module service_cap() {
    co = chassis_bore - 0.4;
    difference() {
        cylinder(d = co, h = cap_l);
        translate([0, 0, 2])
            cylinder(d = co - 2 * chassis_wall, h = cap_l + 0.1);
        // cross-hole for a removal hook
        translate([0, 0, 3]) rotate([90, 0, 0])
            cylinder(d = 3, h = co + 2, center = true);
    }
}

// Full 357 mm stack, bottom (z=0) to top. Stations are added in later commits.
module full_stack() {
    section_a();
    section_b();
}

module section_a() {
    difference() {
        union() {
            shell(0, split_z);
            service_cap();
            translate([0, 0, cell_z0 - stop_ring_h]) cell_cradle();
            // ribs: cradle -> shell (angles clear of the +Y C-slot and windows)
            for (a = [0, 150, 270])
                for (z = [cell_z0 + 15, cell_z1 - 15])
                    rib(a, z, cradle_od / 2 - 0.6, chassis_bore / 2 + 0.2);
        }
        // female socket for the segment lap (removes wall only, not the cradle)
        translate([0, 0, split_z - joint_overlap])
            difference() {
                cylinder(d = socket_bore, h = joint_overlap + 0.1);
                cylinder(d = chassis_bore, h = joint_overlap + 0.2);
            }
    }
}

module section_b() {
    shell(split_z, shell_top);
    translate([0, 0, split_z - joint_overlap])
        difference() {
            cylinder(d = spigot_od, h = joint_overlap);
            translate([0, 0, -0.1]) cylinder(d = chassis_bore, h = joint_overlap + 0.2);
        }
    translate([0, 0, pico_z0]) pico_bay();
    translate([0, 0, sleeve_z0]) sleeve_boss();
    translate([0, 0, radio_z0]) radio_mount();
    translate([0, 0, radiator_z0]) antenna_guide();
    // ribs: Pico bay rails -> shell (rails are at x = +/-)
    for (a = [0, 180])
        for (z = [pico_z0 + 12, pico_z1 - 12])
            rib(a, z, 12.1, chassis_bore / 2 + 0.2);
    // ribs: sleeve boss -> shell
    for (a = [0, 150, 270])
        for (z = [sleeve_z0 + 15, sleeve_z1 - 15])
            rib(a, z, sleeve_boss_od / 2 - 0.6, chassis_bore / 2 + 0.2);
    // ribs: radio mount side walls -> shell
    for (a = [0, 180])
        rib(a, radio_z0 + 8, 9.2, chassis_bore / 2 + 0.2);
    // ribs: antenna-guide struts -> shell top ring
    for (a = [0, 120, 240])
        rib(a, shell_top - 1, guide_r - 1.0, chassis_bore / 2 + 0.2);
}

// ---------------------------------------------------------------------------
// Numeric part dispatch (integers only)
// ---------------------------------------------------------------------------
if (part == 0) {
    full_stack();
} else if (part == 1) {
    cell_cradle();
} else if (part == 2) {
    section_a();
} else if (part == 3) {
    section_b();
}

