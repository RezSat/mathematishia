from mathematishia import latex_to_ascii, Parser, Interpreter, tokenizer

def create_ast(expression):
    expression = latex_to_ascii(expression)
    # Tokenize the expression
    tokens = tokenizer(expression)
    # Parse the tokens into an AST
    parser = Parser(tokens)
    return parser.parse()

def simplify(expression):
    ast = create_ast(expression)
    interpreter = Interpreter()
    interpreter.steps_array = []
    result = interpreter.visit(ast.node)
    result_ = str(create_ast(str(result)).node)

    return result_


print(simplify("2+2"))