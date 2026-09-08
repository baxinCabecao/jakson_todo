import React, { useState, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import "./DatePicker.css";

interface DatePickerProps {
  value?: string; // YYYY-MM-DD
  onChange: (newValue: string | undefined) => void;
  children?: React.ReactNode;
}

const toLocalFormat = (dbDate?: string): string => {
  if (!dbDate) return "";
  const parts = dbDate.split("-");
  if (parts.length === 3) {
    const [year, month, day] = parts;
    return `${day}/${month}/${year}`;
  }
  return dbDate;
};

const toDbFormat = (localDate: string): string => {
  if (!localDate) return "";
  const parts = localDate.split("/");
  if (parts.length === 3) {
    const [day, month, year] = parts;
    return `${year}-${month}-${day}`;
  }
  return localDate;
};

const formatToMask = (val: string): string => {
  const digits = val.replace(/\D/g, "").slice(0, 8);
  let formatted = digits;
  if (digits.length > 2) {
    formatted = `${digits.slice(0, 2)}/${digits.slice(2)}`;
  }
  if (digits.length > 4) {
    formatted = `${formatted.slice(0, 5)}/${digits.slice(4)}`;
  }
  return formatted;
};

const isValidDate = (localDate: string): boolean => {
  if (!localDate || localDate.length < 10) return false;
  const parts = localDate.split("/");
  if (parts.length !== 3) return false;
  const day = parseInt(parts[0], 10);
  const month = parseInt(parts[1], 10);
  const year = parseInt(parts[2], 10);

  if (isNaN(day) || isNaN(month) || isNaN(year) || year < 1000) {
    return false;
  }

  const parsedDate = new Date(year, month - 1, day);
  return (
    parsedDate.getFullYear() === year &&
    parsedDate.getMonth() === month - 1 &&
    parsedDate.getDate() === day
  );
};

export const DatePicker: React.FC<DatePickerProps> = ({
  value,
  onChange,
  children,
}) => {
  const [showCalendar, setShowCalendar] = useState(false);
  const [inputValue, setInputValue] = useState(toLocalFormat(value));
  const [calMonth, setCalMonth] = useState(new Date().getMonth());
  const [calYear, setCalYear] = useState(new Date().getFullYear());
  const [coords, setCoords] = useState({ top: 0, left: 0 });
  const datepickerRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  const updateCoords = () => {
    if (datepickerRef.current) {
      const rect = datepickerRef.current.getBoundingClientRect();
      setCoords({
        top: rect.bottom + window.scrollY,
        left: rect.left + window.scrollX,
      });
    }
  };

  useEffect(() => {
    if (showCalendar) {
      updateCoords();
      // Listen to scroll and resize on any container
      window.addEventListener("scroll", updateCoords, true);
      window.addEventListener("resize", updateCoords);
    }
    return () => {
      window.removeEventListener("scroll", updateCoords, true);
      window.removeEventListener("resize", updateCoords);
    };
  }, [showCalendar]);

  const monthNames = [
    "Janeiro", "Fevereiro", "Março", "Abril", "Maio", "Junho",
    "Julho", "Agosto", "Setembro", "Outubro", "Novembro", "Dezembro"
  ];
  const weekDays = ["D", "S", "T", "Q", "Q", "S", "S"];

  // Sync internal input value if external value changes
  useEffect(() => {
    setInputValue(toLocalFormat(value));
  }, [value]);

  // Adjust display month/year when calendar is opened or value is valid
  useEffect(() => {
    if (showCalendar) {
      const activeValue = value || (inputValue && isValidDate(inputValue) ? toDbFormat(inputValue) : "");
      if (activeValue) {
        const parts = activeValue.split("-");
        if (parts.length === 3) {
          const y = parseInt(parts[0], 10);
          const m = parseInt(parts[1], 10) - 1;
          if (!isNaN(m) && !isNaN(y)) {
            setCalMonth(m);
            setCalYear(y);
          }
        }
      }
    }
  }, [showCalendar, value]);

  // Close calendar on outside clicks
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const clickedNode = event.target as Node;
      const isInsideTrigger = datepickerRef.current && datepickerRef.current.contains(clickedNode);
      const isInsidePopover = popoverRef.current && popoverRef.current.contains(clickedNode);

      if (!isInsideTrigger && !isInsidePopover) {
        setShowCalendar(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, []);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = formatToMask(e.target.value);
    setInputValue(val);

    if (val === "") {
      onChange(undefined);
    } else if (isValidDate(val)) {
      onChange(toDbFormat(val));
    }
  };

  const handleBlur = () => {
    if (inputValue && !isValidDate(inputValue)) {
      setInputValue(toLocalFormat(value));
    }
  };

  const handlePrevMonth = (e: React.MouseEvent) => {
    e.preventDefault();
    if (calMonth === 0) {
      setCalMonth(11);
      setCalYear(calYear - 1);
    } else {
      setCalMonth(calMonth - 1);
    }
  };

  const handleNextMonth = (e: React.MouseEvent) => {
    e.preventDefault();
    if (calMonth === 11) {
      setCalMonth(0);
      setCalYear(calYear + 1);
    } else {
      setCalMonth(calMonth + 1);
    }
  };

  const handleSelectDate = (day: number, month: number, year: number, e: React.MouseEvent) => {
    e.preventDefault();
    const pad = (n: number) => n.toString().padStart(2, "0");
    const dbVal = `${year}-${pad(month + 1)}-${pad(day)}`;
    onChange(dbVal);
    setShowCalendar(false);
  };

  const handleSelectToday = (e: React.MouseEvent) => {
    e.preventDefault();
    const today = new Date();
    const pad = (n: number) => n.toString().padStart(2, "0");
    const todayStr = `${today.getFullYear()}-${pad(today.getMonth() + 1)}-${pad(today.getDate())}`;
    onChange(todayStr);
    setShowCalendar(false);
  };

  const handleClearDate = (e: React.MouseEvent) => {
    e.preventDefault();
    onChange(undefined);
    setShowCalendar(false);
  };

  const getDaysArray = () => {
    const firstDayIndex = new Date(calYear, calMonth, 1).getDay();
    const daysInMonth = new Date(calYear, calMonth + 1, 0).getDate();
    const daysInPrevMonth = new Date(calYear, calMonth, 0).getDate();

    const cells = [];

    for (let i = firstDayIndex - 1; i >= 0; i--) {
      cells.push({
        day: daysInPrevMonth - i,
        month: calMonth === 0 ? 11 : calMonth - 1,
        year: calMonth === 0 ? calYear - 1 : calYear,
        isCurrentMonth: false,
      });
    }

    for (let i = 1; i <= daysInMonth; i++) {
      cells.push({
        day: i,
        month: calMonth,
        year: calYear,
        isCurrentMonth: true,
      });
    }

    const remaining = 42 - cells.length;
    for (let i = 1; i <= remaining; i++) {
      cells.push({
        day: i,
        month: calMonth === 11 ? 0 : calMonth + 1,
        year: calMonth === 11 ? calYear + 1 : calYear,
        isCurrentMonth: false,
      });
    }

    return cells;
  };

  const now = new Date();
  const todayDay = now.getDate();
  const todayMonth = now.getMonth();
  const todayYear = now.getFullYear();

  return (
    <div className="datepicker-container" ref={datepickerRef}>
      {children ? (
        <div className="datepicker-trigger" onClick={() => setShowCalendar(!showCalendar)}>
          {children}
        </div>
      ) : (
        <input
          type="text"
          placeholder="DD/MM/AAAA"
          maxLength={10}
          value={inputValue}
          onChange={handleInputChange}
          onFocus={() => setShowCalendar(true)}
          onBlur={handleBlur}
        />
      )}

      {showCalendar && createPortal(
        <div
          ref={popoverRef}
          className="datepicker-popover"
          style={{
            position: "absolute",
            top: `${coords.top}px`,
            left: `${coords.left}px`,
            zIndex: 9999,
          }}
        >
          <div className="datepicker-header">
            <button type="button" className="datepicker-nav-btn" onClick={handlePrevMonth}>
              &lt;
            </button>
            <span className="datepicker-month-year">
              {monthNames[calMonth]} {calYear}
            </span>
            <button type="button" className="datepicker-nav-btn" onClick={handleNextMonth}>
              &gt;
            </button>
          </div>
          <div className="datepicker-weekdays">
            {weekDays.map((wd, idx) => (
              <span key={idx} className="datepicker-weekday">
                {wd}
              </span>
            ))}
          </div>
          <div className="datepicker-grid">
            {getDaysArray().map((cell, idx) => {
              const pad = (n: number) => n.toString().padStart(2, "0");
              const cellDateStr = `${cell.year}-${pad(cell.month + 1)}-${pad(cell.day)}`;
              const isSelected = value === cellDateStr;
              const isToday = cell.day === todayDay && cell.month === todayMonth && cell.year === todayYear;
              return (
                <button
                  key={idx}
                  type="button"
                  className={`datepicker-day-btn ${cell.isCurrentMonth ? "current" : "other"} ${
                    isToday ? "today" : ""
                  } ${isSelected ? "selected" : ""}`}
                  onClick={(e) => handleSelectDate(cell.day, cell.month, cell.year, e)}
                >
                  {cell.day}
                </button>
              );
            })}
          </div>
          <div className="datepicker-footer">
            <button type="button" className="btn-datepicker-today" onClick={handleSelectToday}>
              Hoje
            </button>
            <button type="button" className="btn-datepicker-clear" onClick={handleClearDate}>
              Sem Vencimento
            </button>
          </div>
        </div>,
        document.body
      )}
    </div>
  );
};
