from setuptools import setup, find_packages
import codecs
import os

VERSION = '0.0.3'
DESCRIPTION = 'A Computer Algebra System, which computes step-by-step solutions to math problems'

# Setting up
setup(
    name="mathematishia",
    version=VERSION,
    author="RezSat (Yehan Wasura)",
    author_email="<wasurayehan@gmail.com>",
    description=DESCRIPTION,
    license_files = ('LICENSE',),
    packages=find_packages(),
    install_requires=['sympy'],
    keywords=['python', 'CAS', 'computer algebra system', 'step-by-step', 'maths','mathematishia', 'mathsteps'],
    classifiers=[
        "Development Status :: 1 - Planning",
        "Intended Audience :: Developers",
        "Programming Language :: Python :: 3",
        "Operating System :: Unix",
        "Operating System :: MacOS :: MacOS X",
        "Operating System :: Microsoft :: Windows",
    ]
)