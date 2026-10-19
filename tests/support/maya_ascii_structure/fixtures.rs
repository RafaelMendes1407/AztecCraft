pub const HEADER: &str = concat!(
    "//Maya ASCII 2026 scene\n",
    "requires maya \"2026\";\n",
    "currentUnit -l centimeter -a degree -t film;\n",
    "fileInfo \"AztecCraft\" \"1\";\n",
);
pub const GEOMETRY: &str = concat!(
    "createNode transform -n \"plain\";\n",
    "createNode mesh -n \"plainShape\" -p \"plain\";\n",
    "\tsetAttr -s 4 \".vt[0:3]\"\n",
    "\t-0.5 0 -0.5\n\t-0.5 0 0.5\n\t0.5 0 0.5\n\t0.5 0 -0.5\n;\n",
    "\tsetAttr -s 4 \".ed[0:3]\"\n",
    "\t0 1 0\n\t1 2 0\n\t2 3 0\n\t3 0 0\n;\n",
    "\tsetAttr -s 1 \".fc[0:0]\" -type \"polyFaces\"\n",
    "\tf 4 0 1 2 3\n;\n",
);
pub const MATERIALS: &str = concat!(
    "createNode lambert -n \"red\";\n",
    "\tsetAttr \".c\" -type \"float3\" -0 0.1 1;\n",
    "createNode shadingEngine -n \"redSG\";\n",
    "createNode lambert -n \"unused\";\n",
    "\tsetAttr \".c\" -type \"float3\" 0.5 0.5 0.5;\n",
    "createNode shadingEngine -n \"unusedSG\";\n",
);
pub const CONNECTIONS: &str = concat!(
    "connectAttr \"red.oc\" \"redSG.ss\";\n",
    "connectAttr \"unused.oc\" \"unusedSG.ss\";\n",
    "connectAttr \"plainShape.iog\" \"redSG.dsm\" -na;\n",
);

pub fn golden() -> String {
    format!("{HEADER}{GEOMETRY}{MATERIALS}{CONNECTIONS}")
}
