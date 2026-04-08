"""Mechanical split of src/parser/mod.rs into 5 focused files."""
import re, sys, os

SRC = r'C:\workspace\PragmaWorks\loom\src\parser\mod.rs'
PARSER_DIR = r'C:\workspace\PragmaWorks\loom\src\parser'

with open(SRC, 'r', encoding='utf-8') as f:
    raw = f.readlines()

# Helper: 1-indexed inclusive range → list of lines
def L(start, end):
    return raw[start-1:end]

# ── Section ranges (1-indexed, inclusive) ───────────────────────────────────
# These come from careful reading of the file.

BEING_RANGES = [
    (1459, 1500),   # parse_annotation_decl (with M66b comment)
    (1502, 1595),   # parse_correctness_report (with M67 comment)
    (1619, 2348),   # parse_being_def
    (2350, 2443),   # parse_migration_block
    (2445, 2524),   # parse_journal_block
    (2526, 2600),   # parse_scenario_block
    (2628, 2738),   # parse_usecase_block
    (2740, 2775),   # collect_usecase_field_value
    (2922, 2955),   # parse_provides_block + parse_requires_block
    (3955, 4035),   # parse_degenerate_block + parse_canalization_block
    (4107, 4152),   # parse_senescence_block + parse_adopt_decl
    (4154, 4218),   # parse_criticality_block
    (4220, 4263),   # parse_niche_construction
    (4265, 4334),   # parse_sense_def
    (5046, 5108),   # parse_boundary_block
    (5110, 5189),   # parse_cognitive_memory_block
]

TYPES_RANGES = [
    (3137, 3485),   # all type methods (parse_type_or_refined..parse_contract)
]

EXPR_RANGES = [
    (3487, 3953),   # all expression methods (parse_expr..parse_lambda)
]

ITEMS_RANGES = [
    (515,  640),    # parse_invariant, parse_test_def, parse_interface_def, parse_lifecycle_def
    (642,  721),    # parse_temporal_def
    (723,  788),    # parse_separation_block
    (790,  870),    # parse_gradual_block + parse_distribution_block
    (872,  1010),   # family_from_model_string + parse_distribution_family
    (1012, 1102),   # parse_stochastic_process_block
    (1104, 1134),   # parse_timing_safety_block
    (1136, 1223),   # parse_proposition_def, parse_functor_def, parse_monad_def, parse_certificate_def
    (1301, 1457),   # parse_aspect_def + pointcut methods
    (2777, 2920),   # parse_ecosystem_def
    (3001, 3135),   # parse_fn_def
    (4037, 4105),   # parse_pathway_def + parse_symbiotic_import
    (4336, 4422),   # parse_store_def
    (4424, 4459),   # parse_store_kind
    (4461, 4511),   # parse_store_table_entry..parse_store_edge_entry
    (4513, 4580),   # parse_store_fact_entry..parse_store_mapreduce_entry
    (4582, 4636),   # parse_store_consumer_entry, parse_mapreduce_sig_as_string, token_as_display_string
    (4668, 4732),   # parse_store_config_value, parse_tensor_rank, parse_tensor_shape
    (4734, 5030),   # parse_session_def..parse_effect_handler
    (5032, 5044),   # comments (M109, M103) before parse_boundary_block — skip, boundary goes to being
    (5192, 5245),   # parse_property_block + collect_property_expr
]

# Remove the placeholder comment range that doesn't contain real methods
ITEMS_RANGES = [r for r in ITEMS_RANGES if r != (5032, 5044)]

# ── Build content for each file ─────────────────────────────────────────────

SUBMOD_HEADER = """\
use crate::ast::*;
use crate::error::LoomError;
use crate::lexer::Token;

impl<'src> crate::parser::Parser<'src> {
"""

SUBMOD_FOOTER = "}\n"


def build_submod(ranges, filename):
    parts = [SUBMOD_HEADER]
    for (s, e) in ranges:
        chunk = L(s, e)
        parts.extend(chunk)
        # Ensure trailing newline between chunks
        if parts and not parts[-1].endswith('\n'):
            parts.append('\n')
        parts.append('\n')
    parts.append(SUBMOD_FOOTER)
    content = ''.join(parts)
    # Fix token_to_source calls: bare call (not self.) → super::token_to_source
    content = re.sub(r'(?<!\bsuper::)(?<!\bcrate::parser::)\btoken_to_source\(', 'super::token_to_source(', content)
    # Fix token_keyword_str calls (if any in submodules)
    content = re.sub(r'(?<!\bsuper::)(?<!\bcrate::parser::)\btoken_keyword_str\(', 'super::token_keyword_str(', content)
    path = os.path.join(PARSER_DIR, filename)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f'Created {path} ({len(content.splitlines())} lines)')


# ── Build new mod.rs ────────────────────────────────────────────────────────

def build_mod_rs():
    # Collect the set of lines used in submodules so we know what to REMOVE from mod.rs
    used = set()
    for ranges in [BEING_RANGES, TYPES_RANGES, EXPR_RANGES, ITEMS_RANGES]:
        for (s, e) in ranges:
            for i in range(s, e+1):
                used.add(i)

    # Lines to KEEP in mod.rs (1-indexed)
    # Strategy: keep line if it's not in `used` AND it's not beyond the impl close (5246)
    # But we need to be careful: we keep lines 1..513, then specific ranges, then tests.
    # Easier: build explicit keep ranges:
    MOD_KEEP = [
        (1, 513),       # header through parse_module
        (1225, 1299),   # parse_optional_type_params + parse_value_as_string
        (1597, 1617),   # parse_flow_label
        (2602, 2626),   # collect_rest_of_line
        (2957, 2999),   # parse_item
        (4638, 4666),   # parse_inline_fields
        # 5246 is just `}` — the impl block closer
        # We'll add it explicitly
        (5248, 5361),   # tests + free functions
    ]

    parts = []
    # Lines 1-22: as-is
    parts.extend(L(1, 22))
    parts.append('\n')
    # Add submodule declarations
    parts.append('mod being;\n')
    parts.append('mod types_parser;\n')
    parts.append('mod expressions;\n')
    parts.append('mod items;\n')
    parts.append('\n')
    # Lines 24-513: impl block start through parse_module
    parts.extend(L(24, 513))
    parts.append('\n')
    # parse_optional_type_params + parse_value_as_string
    parts.extend(L(1225, 1299))
    parts.append('\n')
    # parse_flow_label
    parts.extend(L(1597, 1617))
    parts.append('\n')
    # collect_rest_of_line
    parts.extend(L(2602, 2626))
    parts.append('\n')
    # parse_item
    parts.extend(L(2957, 2999))
    parts.append('\n')
    # parse_inline_fields (with preceding comment)
    # Actually let's also include the comment block before parse_inline_fields
    # Line 4638 starts the doc comment for parse_inline_fields
    parts.extend(L(4638, 4666))
    parts.append('}\n')  # close impl block
    parts.append('\n')
    # Tests + free functions (5248-5361)
    parts.extend(L(5248, 5361))

    content = ''.join(parts)
    with open(SRC, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f'Rewrote {SRC} ({len(content.splitlines())} lines)')


# ── Execute ──────────────────────────────────────────────────────────────────

build_submod(BEING_RANGES, 'being.rs')
build_submod(TYPES_RANGES, 'types_parser.rs')
build_submod(EXPR_RANGES, 'expressions.rs')
build_submod(ITEMS_RANGES, 'items.rs')
build_mod_rs()

print('Done.')
