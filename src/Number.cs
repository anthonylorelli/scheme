namespace Scheme {
    public class Complex {
        private double _real;
        private double _imag;

        public Complex(double real, double imag) {
            _real = real; _imag = imag;
        }

        public double Real {
            get { return _real; }
        }

        public double Imag {
            get { return _imag; }
        }

        public override string ToString() {
            string s = _real.ToString();
            if ( _imag > 0 ) s += "+";
            s += _imag;
            s += "i";
            return s;
        }
    }
}
