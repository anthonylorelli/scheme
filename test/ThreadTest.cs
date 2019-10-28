//
// Copyright 2006 Anthony J. Lorelli
// $Id: ThreadTest.cs 290 2006-03-14 22:56:07Z  $
//
using System;
using System.Threading;

public class ThreadTest {
    public static void Empty() { }

    public static void PrintHello() {
        System.Console.WriteLine("Hello");
    }

    public static void Main(string[] args) {
        if (args.Length != 1) {
            System.Console.WriteLine("Usage: threadtest.exe <numthreads>");
        } else {
            int n = int.Parse(args[0]);
            for (int i = 0; i < n; ++i) {
                Thread t = new Thread(new ThreadStart(Empty));
                t.Start();
            }
        }
    }
}
