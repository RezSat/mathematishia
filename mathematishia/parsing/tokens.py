CARROT = 32
DOT = 23
ENCODING = 63
ENDMARKER = 0
EQUAL = 22
GREATER = 21
GREATEREQUAL = 30
LBRACE = 25
LESS = 20
LESSEQUAL = 29
LPAR = 7
LSQB = 9
MDOT = "⋅" # custom
MINUS = 15
NAME = 1
NOTEQUAL = 28
NUMBER = 2
OP = 54 #OPERATION COMMAN
PERCENT = 24
PLUS = 14
RPAR = 8
RSQB = 10
SLASH = 17
STAR = 16

NEWLINE = 4

UNARY_OPERATORS = {MINUS, PLUS}
BINARY_OPERATORS = {MINUS, PLUS, STAR, CARROT, SLASH}
TRIG_FUNCS = ['sin', 'cos', 'tan', 'cosec', 'csc', 'cot', 'sec']
OTHER_FUNCS = ['lim', 'hcf', 'lcm', 'log']

DESCRIPTIONS = {
 	# BASIC OPERATIONS

	'add': 'Add {} to {} = {}',
	'sub': 'Subtract {} from {} = {}',
	'mul': 'Multiply {} and {} = {}',
	'div': 'Divide {} by {} = {}',
	'exp': '{} to the power of {} = {}',
	"paran": 'Evaluate within parenthesis: ({}) = {}',

	# MATHEMATICAL ERRORS
	"zero_division_eror": "Divide by Zero is Undefined."

	# TRIG IDENTITIES & FORMULAS

	# ALGEBRAIC LAWS

	# LOG RULES
}

STEP_FORMAT = {
	"description": None,
	"step": None,
	"result": None,
	"sub_steps": [],
	"latex_format": None,
}